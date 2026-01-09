mod buffer_pair;
mod completions;
pub mod grammar;
mod recognizer;
mod set_buffers;
use std::borrow::Borrow;

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
pub fn trace_to_ast<'c, Symbol: CfgSymbol>(
    cfg: &'c crate::grammar::Cfg<Symbol>,
    src: &[Symbol::Terminal],
    trace: &[(usize, usize, NtSymbol)],
    init_sym: &'c Symbol,
) -> Ast<'c, Symbol> {
    let mut ast = vec![];
    let mut stack = vec![(src, init_sym)];
    'next_node: while let Some((span, state)) = stack.pop() {
        // println!("{state:?} under {:?}", stack.iter().map(|(_, s)| s).collect::<Vec<_>>());
        let state_nt = match state.as_part() {
            Either::Err(sym) => sym,
            Either::Ok(_) => panic!("terminal in trace"),
        };
        let rules = cfg.rules_for(state_nt);
        let start = span.as_ptr() as usize - src.as_ptr() as usize;
        for rule in rules {
            let stack_len = stack.len();
            if matched_rule(span, start, trace, &rule.parts, &mut stack, state_nt, span.len()) {
                // if let Some((_, first_child)) = (stack_len < stack.len())
                //     .then_some(())
                //     .and_then(|_| stack.last())
                //     && let Err(new_nt) = first_child.as_part()
                //     && new_nt == state_nt
                // {
                //     // left-recursive without progress
                //     // FIXME: This can actually happen recursively so need to add handling for that too
                // } else {

                    // push nodes to ast
                    let end = start + span.len();
                    ast.push(Node {
                        transition: state,
                        start,
                        end,
                        children: stack.len() - stack_len,
                    });
                    continue 'next_node;
                // }
            }
            stack.truncate(stack_len);
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
fn matched_rule<'a, 'c, Symbol: CfgSymbol>(
    mut src: &'a [Symbol::Terminal],
    offset: usize,
    mut trace: &[(usize, usize, NtSymbol)],
    rule: &'c [Symbol],
    children: &mut Vec<(&'a [Symbol::Terminal], &'c Symbol)>,
    parent_sym: u32,
    parent_len: usize,
) -> bool {
    // println!("matching rule {:?}", rule);
    for part in rule.iter().rev() {
        match part.as_part() {
            Either::Ok(part) => {
                if src.last() != Some(part.borrow()) {
                    return false;
                }
                src = &src[..src.len() - 1];
            }
            Either::Err(sym) => {
                // find the last occurrence of this symbol in the trace that ends at src_index + 1
                let Some((start, end, _)) = trace
                    .iter()
                    .rfind(|&&(start, end, s)|
                        s == sym && end == (src.len() + offset)
                        // FIXME: This needs to work recursively again,
                        // if a rule is left/right recursive but hidden through another rule
                        && (s != parent_sym || (end - start) < parent_len)
                    )
                else {
                    return false;
                };
                // println!("pushing {start}..{end} for sym {sym}");
                children.push((&src[*start - offset..*end - offset], part));
                src = &src[..start - offset];
                let i = trace.partition_point(|(_, match_end, _)| match_end <= start);
                trace = &trace[..i];
            }
        }
    }
    src.len() == 0
}
#[derive(Debug)]
pub struct Node<'c, Symbol> {
    // FIXME: Adding this lifetime is silly. switch later
    transition: &'c Symbol,
    start: usize,
    end: usize,
    children: usize,
    // parent: usize,
    // // next_sibling: usize,
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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
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
    type TerminalRef<'a> = Utf8SingleByte
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
        self.label.fmt(f)
    }
}
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
