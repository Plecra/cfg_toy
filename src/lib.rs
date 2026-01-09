mod buffer_pair;
mod completions;
pub mod grammar;
mod set_buffers;
mod recognizer;
pub use recognizer::{parse_earley, Trace};


pub struct Node {
    // transition: u32,
    // start: usize,
    // end: usize,
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
