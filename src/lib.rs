mod buffer_pair;
mod completions;
pub mod grammar;
mod recognizer;
mod set_buffers;
pub use recognizer::{Trace, parse_earley};

// Build an AST for a trace of an unambiguous parse.
// That is, a parse where all ambiguities have been resolved, in examples like
// S ::= "then"
// S ::= "that"
//
// It's fine that the transition is ambiguous with arbitrary lookahead,
// because it'll be resolved by the time we need to build the AST: one of the two will have been invalid
// The trace should be sorted by (start, sym). This is not inherentely true of the trace we build during parsing.
// It's sorted by `end` and arbitrary sym order.
pub fn trace_to_ast(
    cfg: &crate::grammar::Cfg,
    src: &[u8],
    trace: &[(usize, usize, NtSymbol)],
    init_sym: u32,
) -> Ast {
    let mut ast = vec![];
    let mut stack = vec![(src, init_sym)];
    'next_node: while let Some((span, state)) = stack.pop() {
        println!("{state:?} under {stack:?}");
        let rules = cfg.rules_for(state);
        let start = span.as_ptr() as usize - src.as_ptr() as usize;
        for rule in rules {
            let stack_len = stack.len();
            println!("{state:?}: {rule:?}");
            if matched_rule(span, start, trace, &rule.parts, &mut stack) {
                println!("found rule: {:?}", &stack[stack_len..]);
                if let Some((_, first_child)) = dbg!(
                    (stack_len < stack.len())
                        .then_some(())
                        .and_then(|_| stack.last())
                ) && *first_child == state
                {
                    // left-recursive without progress
                    // FIXME: This can actually happen recursively so need to add handling for that too
                    continue;
                }
                // push nodes to ast
                let end = start + span.len();
                ast.push(Node {
                    transition: state,
                    start,
                    end,
                    children: stack.len() - stack_len,
                });
                println!("wat {stack:?}");
                continue 'next_node;
            }
            stack.truncate(stack_len);
        }
        panic!("no matching rule found");
    }
    ast
}
fn matched_rule<'a>(
    mut src: &'a [u8],
    offset: usize,
    mut trace: &[(usize, usize, NtSymbol)],
    rule: &[u32],
    children: &mut Vec<(&'a [u8], NtSymbol)>,
) -> bool {
    for &part in rule.iter().rev() {
        if part < 256 {
            if src.last() != Some(&(part as u8)) {
                return false;
            }
            src = &src[..src.len() - 1];
        } else {
            // nonterminal
            let sym = part;
            // find the last occurrence of this symbol in the trace that ends at src_index + 1
            let Some((start, end, _)) = trace
                .iter()
                .rfind(|&&(_, end, s)| s == sym && end == (src.len() + offset))
            else {
                return false;
            };
            println!("pushing {start}..{end} for sym {sym}");
            children.push((&src[*start - offset..*end - offset], sym));
            src = &src[..start - offset];
            let i = trace.partition_point(|(_, match_end, _)| match_end <= start);
            trace = &trace[..i];
        }
    }
    true
}
#[derive(Debug)]
pub struct Node {
    transition: u32,
    start: usize,
    end: usize,
    children: usize,
    // parent: usize,
    // // next_sibling: usize,
}
type Ast = Vec<Node>;
use recognizer::NtSymbol;
use recognizer::TraceAt;
struct RecordTrace<'a> {
    current_symbol: usize,
    trace: &'a mut Vec<(usize, usize, NtSymbol)>,
}
impl TraceAt for RecordTrace<'_> {
    fn completed(&mut self, back_ref: usize, sym: NtSymbol) {
        self.trace.push((back_ref, self.current_symbol, sym));
    }
}
impl Trace for Vec<(usize, usize, NtSymbol)> {
    fn at(&mut self, symbol_index: usize) -> impl TraceAt + '_ {
        RecordTrace {
            current_symbol: symbol_index,
            trace: self,
        }
    }
}
