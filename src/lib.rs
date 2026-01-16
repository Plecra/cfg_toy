mod buffer_pair;
pub mod completions;
pub mod grammar;
pub mod recognizer;
mod set_buffers;
use std::borrow::Borrow;

pub use recognizer::{Trace, parse_earley};

enum CallFrame<'a, 'c, Symbol: CfgSymbol> {
    ProcessNode(
        &'a [Symbol::Terminal],
        &'a [(usize, usize, NtSymbol, &'c [Symbol])],
        &'c Symbol,
        Option<(Vec<(usize, usize, u32, &'c [Symbol])>, usize)>,
    ),
    ReturnToParent(usize),
}
// Build an AST for a trace of an unambiguous parse.
// That is, a parse where all ambiguities have been resolved, in examples like
// S ::= "then"
// S ::= "that"
//
// It's fine that the transition is ambiguous with arbitrary lookahead,
// because it'll be resolved by the time we need to build the AST: one of the two will have been invalid
// The trace should be sorted by (start, sym). This is not inherentely true of the trace we build during parsing.
// It's sorted by `end` and arbitrary sym order.
pub fn trace_to_ast<'c, Symbol: CfgSymbol + PartialEq>(
    cfg: &'c crate::grammar::Cfg<Symbol>,
    src: &[Symbol::Terminal],
    init_trace: &[(usize, usize, NtSymbol, &'c [Symbol])],
    completions: &crate::completions::Completions<'c, Symbol>,
    init_sym: &'c Symbol,
) -> Ast<'c, Symbol> {
    let mut ast: Ast<'c, Symbol> = vec![];

    // This virtual stack is used to speculatively visit children,
    // and allow it to be aborted with `stack.truncate()` if a rule fails to match.
    // It makes this function look way more complicated! It's not really necessary
    // for the logic, just premature "optimization" :D
    let mut stack = vec![CallFrame::ProcessNode(src, init_trace, init_sym, None)];
    // println!("{trace:?}");
    'next_node: while let Some(res) = stack.pop() {
        let (span, trace_slice, state, reconstructed) = match res {
            CallFrame::ProcessNode(span, trace_slice, state, reconstructed) => (span, trace_slice, state, reconstructed),
            CallFrame::ReturnToParent(idx) => {
                let current = ast.len();
                let x: &mut Node<'c, Symbol> = &mut ast[idx];
                x.transitive_children = current - idx - 1;
                continue;
            }
        };
        // println!("{state:?} under {:?}", stack.iter().map(|(_, s)| s).collect::<Vec<_>>());
        let state_nt = match state.as_part() {
            Either::Err(sym) => sym,
            Either::Ok(_) => panic!("terminal in trace"),
        };
        let rules = cfg.rules_for(state_nt);
        let start = span.as_ptr() as usize - src.as_ptr() as usize;
        stack.push(CallFrame::ReturnToParent(ast.len()));
        let stack_len = stack.len();
        // TODO: The Trace is recording the rules directly now, searching isnt necessary
        for rule in rules {
            stack.truncate(stack_len);
            // println!(
            //     "  trying rule {state:?}{:?} for span {:?}",
            //     rule.parts, span
            // );
            // Here we implement the disambiguation semantics.
            // Whichever rules we visit first here will immediately be selected and we continue,
            // so the ordering of the rules from the cfg gives priority.
            // However in the current implementation this is right-to-left. The difference from left-to-right
            // is quite subtle and only sometimes observable.
            //
            // if ::= "if" cond "then" stmt
            // if ::= "if" cond "then" stmt ("else" stmt)?
            // if a then b else if c then d else e
            //
            // by applying from the right, this disambiguation greedily selects
            // (if c then d else e), insteadof ... oh lmao ltr does the same thing for
            // for a different reason
            //
            // expr ::= expr "<" expr
            // expr ::= expr ">" expr
            // expr ::= expr "<" expr ">"
            // expr ::= expr "(" expr ")"
            // expr ::= "(" expr ")"
            //
            // f<a>(b)
            //
            // from the right: start trying to form a call, find a generic.
            // from the left: comparison, then rhs is another comparison with a parenthesized b.
            //
            // great! actually pinned it down. So a goal here would be to also efficiently implement
            // ltr disambiguation. And the critical challenge is that our data is naturally
            // asymmetrical: The information about `end`s comes from the Trace, and the paired
            // starts comes from the completions.
            //
            // whether that'll be an issue is to be seen! I'll have to try the implementation
            // println!("trying rule {:?} for span {:?}", rule.parts, span);
            if matched_rule(
                span,
                start,
                trace_slice,
                completions,
                &rule.parts,
                &mut stack,
                state_nt,
                span.len(),
                // FIXME: cloning this is definitely silly
                reconstructed.clone(),
            ) {
                if let Some(CallFrame::ProcessNode(new_span, _, first_child, _)) =
                    stack.last().filter(|_| stack_len + 1 == stack.len())
                    && let Err(new_nt) = first_child.as_part()
                    && new_nt == state_nt
                    && new_span.len() == span.len()
                {
                    // left-recursive without progress
                    // FIXME: This can actually happen recursively so need to add handling for that too
                    // I think we might be able to use the same trick from eta rules though: we can always
                    // jump through identity definitions
                } else {
                    // push nodes to ast
                    let end = start + span.len();
                    ast.push(Node {
                        transition: &rule.parts,
                        start,
                        end,
                        children: stack.len() - stack_len,
                        transitive_children: 0,
                    });
                    continue 'next_node;
                }
            }
            // println!("  rule failed");
        }
        panic!("no matching rule found");
    }
    ast
}
type Either<L, R> = std::result::Result<L, R>;

/// This is a very blunt approach just to line all the types
/// up right for making the original (u8, u32) version generic
pub trait CfgSymbol: std::fmt::Debug {
    type Terminal: PartialEq + std::fmt::Debug;
    type TerminalRef<'a>: std::borrow::Borrow<Self::Terminal>
    where
        Self: 'a;
    fn as_part(&self) -> Either<Self::TerminalRef<'_>, NtSymbol>;
}
impl CfgSymbol for u32 {
    type Terminal = u8;
    type TerminalRef<'a> = u8;
    fn as_part(&self) -> Either<Self::TerminalRef<'_>, NtSymbol> {
        if *self < 256 {
            Either::Ok(*self as u8)
        } else {
            Either::Err(*self)
        }
    }
}
type ReconstructedTraceRhs<'c, Symbol> = Option<(Vec<(usize, usize, u32, &'c [Symbol])>, usize)>;
#[allow(clippy::too_many_arguments)]
fn matched_rule<'a, 'c, Symbol: CfgSymbol + PartialEq>(
    mut src: &'a [Symbol::Terminal],
    offset: usize,
    mut trace: &'a [(usize, usize, NtSymbol, &'c [Symbol])],
    completions: &crate::completions::Completions<'c, Symbol>,
    rule: &'c [Symbol],
    children: &mut Vec<CallFrame<'a, 'c, Symbol>>,
    parent_sym: u32,
    parent_len: usize,
    reconstructed: ReconstructedTraceRhs<'c, Symbol>,
) -> bool {
    // In the current algorithm it's possible to see a match for the RHS of a
    // rule, which is *actually* the child of another higherlevel rule which
    // led to a failed parse because it returned "too late"
    // this form of ambiguity could cause a match on a rule to fail, because
    // the wrong path was taken. This means the reconstruction must
    // attempt nondeterminism whenever selecting the match that we'll use for
    // a nonterminal.
    //
    // This can happen even in unambiguous parses.
    // println!("checking rule {:?} against src {offset:?} {:?} {:?} {trace:?}", rule, offset + src.len(), reconstructed);
    let mut iter = rule.iter();
    while let Some(part) = iter.next_back() {
        match part.as_part() {
            Either::Ok(part) => {
                if src.last() != Some(part.borrow()) {
                    return false;
                }
                src = &src[..src.len() - 1];
            }
            Either::Err(sym) => {
                let end_loc = offset + src.len();
                // let mut tests = 0;
                // find the last occurrence of this symbol in the trace that ends at src_index + 1
                let Some((start, end, reconstructed)) = 
                    (
                        reconstructed.as_ref().and_then(|recon_trace|
                        recon_trace.0[..recon_trace.1]
                            .iter()
                            .rev()
                            .take_while(|&&(_, match_end, _, _)| end_loc <= match_end)
                            // .inspect(|c| println!("reconstructed check: {:?}", c))
                            .find_map(|&(start, end, s, _)| 
                                (s == sym && end == end_loc
                                // FIXME: This needs to work recursively again,
                                // if a rule is left/right recursive but hidden through another rule
                                && (s != parent_sym || (end - start) < parent_len)
                                && {
                                    
                                    let range = completions.query_range(start, sym);
                                    completions.completions[range]
                                        .iter()
                                        .any(|c| {
                                            let (back_ref, sym, src_rule, rem) = c.1.clone();
                                            back_ref ==  offset && sym == parent_sym
                                            // The completion must cover the nodes we've parsed so far
                                            && src_rule == rule
                                        })
                                }
                            ).then(|| 
                                {
                                    // println!("used reconstruction :/");
                                    (start, end, reconstructed.clone())
                            })))
                    ).or_else(|| {
                        // println!("checking the actual trace?");
                        trace
                        .iter()
                        .rev()
                        .take_while(|&&(_, match_end, _, _)| end_loc <= match_end)
                        .find_map(|&(start, end, s, _)| {
                            ((({
                            true
                        }) &&
                        (s == sym && end == end_loc
                            // FIXME: This needs to work recursively again,
                            // if a rule is left/right recursive but hidden through another rule
                            && (s != parent_sym || (end - start) < parent_len)
                            && completions
                                .query_original_without_cache_update(start, sym)
                                .any(|st| st.back_ref ==  offset && st.sym == parent_sym
                                    // The completion must cover the nodes we've parsed so far
                                    && st.remaining == &rule[iter.as_slice().len() + 1..]
                                )
                            )).then(|| {
                                // println!("using it!");
                                (start, end, None)
                            })).or_else(|| {
                                // FIXME: scanning completions for every trace entry is really inefficient, the trace
                                // should probably remember the actual completions that were triggered? granted, this
                                // is also only applicable for the right recursive rules that could be children of the
                                // rule we're searching for.
                                let in_bypass = end == end_loc
                                    && completions.query_without_cache_update(start, s).any(|st| {
                                    st.back_ref ==  offset && st.sym == parent_sym
                                        // The completion must cover the nodes we've parsed so far
                                        && st.remaining == &rule[iter.as_slice().len() + 1..]
                                        && st.rule == rule
                                });
                                // println!("need to check bypass: {in_bypass:?} {offset:?} {parent_sym:?}");
                                in_bypass.then(|| {
                                    let got_it = (start, s);
                                    // We've detected that the recognizer used the right recursion path here.
                                    let mut reconstructed_trace = vec![];
                                    let mut todo = vec![got_it];
                                    while let Some((start, sym)) = todo.pop() {
                                        // let nodes_completed_by =
                                        // completions.query_without_cache_update(start, sym).filter(|st| {
                                        //     st.back_ref ==  offset && st.sym == parent_sym
                                        //         // The completion must cover the nodes we've parsed so far
                                        //         && st.remaining == &rule[iter.as_slice().len() + 1..]
                                        // });
                                        let range = completions.query_range(start, sym);
                                        let nodes_completed_by = completions.completions[range]
                                            .iter()
                                            .filter_map(|c| {
                                                let (back_ref, sym, src_rule, rem) = c.1.clone();
                                                match rem {
                                                    crate::completions::Remaining::EmptyAndForwardingTo(start, end) => {
                                                        completions.forwarding_records[start..end].iter()
                                                            .any(|st| {
                                                                st.back_ref ==  offset && st.sym == parent_sym
                                                                    // The completion must cover the nodes we've parsed so far
                                                                    && st.remaining == &rule[iter.as_slice().len() + 1..]

                                                            }).then_some((back_ref, sym, src_rule))
                                                    }
                                                    _ => None,
                                                }
                                            });
                                        for (back_ref, sym, src_rule) in nodes_completed_by {
                                            reconstructed_trace.push((back_ref, end_loc, sym, src_rule));
                                            // println!("{back_ref:?} {end_loc:?} {sym:?} {src_rule:?}");
                                            todo.push((back_ref, sym));
                                            // if reconstructed_trace.len() > 60 {
                                            //     break;
                                            // }
                                        }
                                        todo.sort_by_key(|a| a.0);
                                        todo.dedup_by_key(|a| a.0);
                                    }
                                    reconstructed_trace.sort_by_key(|a| (-(a.0 as isize), a.2));
                                    println!("{got_it:?} {reconstructed_trace:?} {end_loc:?} {sym:?}");
                                    let i = reconstructed_trace.partition_point(|(_, end, _, _)| *end <= end_loc);
                                    let &(start, _, _, _) = reconstructed_trace[..i].iter().rev().find(|(_, end, nt, _)| {
                                        *end == end_loc && *nt == sym
                                    }).unwrap();

                                    (start, end_loc, Some((reconstructed_trace, i)))
                                })
                            })
                        })
                    })
                else {
                    return false;
                };
                // println!("accepted {:?}", start..end);
                let i = trace.partition_point(|(_, match_end, _, _)| *match_end <= end);
                // println!("pushing {start}..{end} for sym {sym}");
                children.push(CallFrame::ProcessNode(
                    &src[start - offset..end - offset],
                    &trace[..i],
                    part,
                    reconstructed
                ));
                trace = &trace[..i];
                src = &src[..start - offset];
                let i = trace.partition_point(|(_, match_end, _, _)| *match_end <= start);
                trace = &trace[..i];
            }
        }
    }
    src.is_empty()
}
#[derive(Debug)]
pub struct Node<'c, Symbol> {
    // FIXME: Adding this lifetime is silly. switch later
    pub transition: &'c [Symbol],
    pub start: usize,
    pub end: usize,
    pub children: usize,
    pub transitive_children: usize,
    // parent: usize,
    // // next_sibling: usize,
}
pub fn print_ast<'a, 'c, S: CfgSymbol + PartialEq>(ast: &'a [Node<'c, S>], indent: usize) {
    print!("{:?}", DebugIt(ast, indent));
    struct DebugIt<'a, S: CfgSymbol>(&'a [Node<'a, S>], usize);
    impl<S> std::fmt::Debug for DebugIt<'_, S>
    where
        S: CfgSymbol + PartialEq,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let ast = self.0;
            let indent = self.1;
            let node = &ast[0];
            for _ in 0..indent {
                write!(f, " ")?;
            }
            write!(f, "- {}..{} ", node.start, node.end)?;
            let mut rule_desc = f.debug_list();
            // Add inlining rules.
            // let mut inlined_children = vec![];
            let mut result_edges = vec![];
            let mut list_terminators = vec![];
            let mut transition = node.transition;
            let mut rem = &ast[1..];
            while !transition.is_empty() {
                let part = &transition[0];
                match part.as_part() {
                    Either::Ok(part) => {
                        rule_desc.entry(part.borrow());
                    }
                    Either::Err(_sym) => {
                        let (child, rest) = rem.split_at(1 + rem[0].transitive_children);
                        if (transition.len() == 1)
                            && result_edges.is_empty()
                            && !child[0].transition.is_empty()
                        {
                            assert_eq!(rest.len(), 0);
                            transition = child[0].transition;
                            rem = &child[1..];
                            continue;
                        } else if child[0].transition.last() == Some(part) && rest.is_empty() {
                            if !transition.is_empty() {
                                list_terminators.push(&transition[1..]);
                            }
                            transition = child[0].transition;
                            rem = &child[1..];
                            continue;
                        } else if child[..child.len() - 1]
                            .iter()
                            .all(|node| node.children == 1)
                        // child.len() == 1 || child.len() == 0
                        {
                            let mut js = vec![0; child.len() - 1];
                            for (i, jo) in js.iter_mut().enumerate() {
                                let leaf = &child[i];
                                let first = match leaf.transition[0].as_part() {
                                    Either::Ok(t) => t,
                                    Either::Err(_) => {
                                        *jo = 1;
                                        continue;
                                    }
                                };
                                rule_desc.entry(&format_args!("^{:?}", first.borrow()));
                                for i in 1..leaf.transition.len() {
                                    let term = match leaf.transition[i].as_part() {
                                        Either::Ok(t) => t,
                                        Either::Err(_) => {
                                            *jo = i + 1;
                                            continue;
                                        }
                                    };
                                    rule_desc.entry(&term.borrow());
                                }
                            }
                            let leaf = &child[child.len() - 1];
                            if leaf.transition.is_empty() {
                                rule_desc.entry(&format_args!("^ε"));
                            } else {
                                let first = &leaf.transition[0];
                                rule_desc.entry(&format_args!("^{:?}", first));
                                for sym in &leaf.transition[1..] {
                                    rule_desc.entry(sym);
                                }
                            }
                            for i in (0..js.len()).rev() {
                                let skip = js[i];
                                let leaf = &child[i];
                                for sym in &leaf.transition[skip..] {
                                    rule_desc.entry(sym);
                                }
                            }
                        } else {
                            rule_desc.entry(part);
                            result_edges.push(child);
                        }
                        rem = rest;
                    }
                }
                transition = &transition[1..];
            }
            while let Some(terminator) = list_terminators.pop() {
                for sym in terminator {
                    rule_desc.entry(sym);
                }
            }
            rule_desc.finish()?;
            writeln!(f)?;
            for emit in result_edges {
                print_ast(emit, indent + 1);
            }
            Ok(())
        }
    }
}
type Ast<'c, Symbol> = Vec<Node<'c, Symbol>>;
use recognizer::NtSymbol;
use recognizer::TraceAt;

pub struct LabelledSymbol {
    pub symbol: NtSymbol,
    pub label: &'static str,
}
impl Ord for LabelledSymbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.symbol.cmp(&other.symbol)
    }
}
impl PartialOrd for LabelledSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for LabelledSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol
    }
}
impl Eq for LabelledSymbol {}
#[derive(PartialEq)]
#[repr(transparent)]
pub struct Utf8SingleByte(u8);
pub fn cast_buf(buf: &[u8]) -> &[Utf8SingleByte] {
    unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const Utf8SingleByte, buf.len()) }
}
impl std::fmt::Debug for Utf8SingleByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", char::from(self.0))
    }
}
impl CfgSymbol for LabelledSymbol {
    type Terminal = Utf8SingleByte;
    type TerminalRef<'a>
        = Utf8SingleByte
    where
        Self: 'a;
    fn as_part(&self) -> Either<Self::TerminalRef<'_>, NtSymbol> {
        if self.symbol < 256 {
            Either::Ok(Utf8SingleByte(self.symbol as u8))
        } else {
            Either::Err(self.symbol)
        }
    }
}
impl std::fmt::Debug for LabelledSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.symbol < 256 {
            char::from(self.symbol as u8).fmt(f)
        } else {
            std::fmt::Display::fmt(&self.label, f)
        }
    }
}
struct RecordTrace<'a, 'c, Symbol> {
    current_symbol: usize,
    trace: &'a mut Vec<(usize, usize, NtSymbol, &'c [Symbol])>,
}
impl<'c, Symbol> TraceAt<'c, Symbol> for RecordTrace<'_, 'c, Symbol> {
    fn completed(&mut self, back_ref: usize, sym: NtSymbol, rule: &'c [Symbol]) {
        self.trace.push((back_ref, self.current_symbol, sym, rule));
    }
}
impl<'c, Symbol> Trace<'c, Symbol> for Vec<(usize, usize, NtSymbol, &'c [Symbol])> {
    fn at(&mut self, symbol_index: usize) -> impl TraceAt<'c, Symbol> + '_ {
        RecordTrace {
            current_symbol: symbol_index,
            trace: self,
        }
    }
}
