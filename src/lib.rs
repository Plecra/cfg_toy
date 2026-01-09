mod buffer_pair;
mod completions;
pub mod grammar;
mod set_buffers;

use buffer_pair::{BufferPair, Transfer};
use completions::{Completions, CompletionsTransaction};
use set_buffers::{grow_ordered_set, isolate_new_elements, sorted_set};

pub struct Node {
    // transition: u32,
    // start: usize,
    // end: usize,
    // parent: usize,
    // // next_sibling: usize,
}
type Ast = Vec<Node>;

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
pub trait TraceAt {
    fn completed(&mut self, back_ref: usize, sym: NtSymbol);
}
pub trait Trace {
    fn at(&mut self, symbol_index: usize) -> impl TraceAt + '_;
}
impl<T: Trace> Trace for &'_ mut T {
    fn at(&mut self, symbol_index: usize) -> impl TraceAt + '_ {
        (**self).at(symbol_index)
    }
}
impl Trace for () {
    fn at(&mut self, _symbol_index: usize) -> impl TraceAt + '_ {}
}
impl TraceAt for () {
    fn completed(&mut self, _back_ref: usize, _sym: NtSymbol) {}
}
// type Symbol = u32;
pub(crate) type NtSymbol = u32;
// TODO: can switch this to encoding the sym id into the slice.
// the standard presentation is that these store (Rule, rule_offset)
pub(crate) type Completion<'a> = (NtSymbol, State<'a>);
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) struct State<'a> {
    back_ref: usize,
    sym: NtSymbol,
    remaining: &'a [u32],
}
fn mk_state(back_ref: usize, sym: NtSymbol, remaining: &[u32]) -> State<'_> {
    State {
        back_ref,
        sym,
        remaining,
    }
}

struct EarleyStep<'c, 'r, T> {
    cfg: &'c grammar::Cfg,
    input_symbol: u8,
    completions_tx: CompletionsTransaction<'c, 'r>,
    next_states: Vec<State<'c>>,
    trace: T,
}
pub fn parse_earley(cfg: &grammar::Cfg, src: &[u8], init_sym: u32, mut trace: impl Trace) {
    let mut states = cfg
        .rules_for(init_sym)
        .map(|r| mk_state(0, init_sym, &r.parts[..]))
        .collect::<Vec<_>>();

    // This is kept between iterations for double buffering to
    // save on allocating it.
    let mut next_states = vec![];

    // TODO: The completions should sorta have a GC pass. Especially for longer files,
    // most of its content is completely unreferenced.
    // Currently using binary search maps due to the need for range queries, it'd be totally sensible
    // to revisit that
    let mut completions = Completions::new(src.len());

    for cursor in 0..src.len() {
        let mut step = EarleyStep {
            cfg,
            input_symbol: src[cursor],
            // The states for the next character get accumulated here, they'll need to be deduplicated
            // before we actually process the next character
            next_states,
            // If any state transition is a prediction, we remember the completion for it to use later
            completions_tx: completions.add_group(),
            trace: trace.at(cursor),
        };
        // As we expand the states, we'll generate more states that need to be processed.
        // we keep track of all generated states here to deduplicate them
        let mut transfer = Transfer {
            states: &states,
            new_states: vec![],
        };
        // To optimize deduplicating the new states, we deduplicate in batches, so that nothing
        // before the current pass needs to be checked again.
        let states_before_pass = transfer.new_states.len();
        // First we transfer out of the states from the last character.
        step.expand_states(&mut transfer);
        let mut new_states = transfer.new_states;

        isolate_new_elements(&mut new_states, states_before_pass);
        grow_ordered_set(&mut new_states, |states| {
            step.expand_states(states);
        });
        sorted_set(&mut step.next_states);

        let mut used_up_states = std::mem::replace(&mut states, step.next_states);
        used_up_states.clear();
        next_states = used_up_states;
    }
    grow_ordered_set(&mut states, |mut states| {
        for i in 0..states.read().len() {
            let state = states.read()[i];
            if state.remaining.is_empty() {
                // This state has recognized its nontermininal starting at state.back_ref
                trace.at(src.len()).completed(state.back_ref, state.sym - 256);
                states.write().extend(completions.query(state.back_ref, state.sym));
                continue;
            }
        }
    });
    // the match state is (back_ref: 0, sym: 256), so will always be at the start
    println!("{:?}", states.first());
    assert_eq!(
        states.first().map(|s| (s.back_ref, s.sym)),
        Some((0, init_sym))
    );
}
impl<'c, T: TraceAt> EarleyStep<'c, '_, T> {
    fn expand_states(&mut self, mut transfer: impl BufferPair<State<'c>>) {
        for i in 0..transfer.read().len() {
            let state = transfer.read()[i];
            self.expand_state(state, transfer.write());
        }
    }
    fn expand_state(&mut self, state: State<'c>, new: &mut Vec<State<'c>>) {
        let Some(&sym) = state.remaining.first() else {
            // This state has recognized its nontermininal starting at state.back_ref
            self.trace.completed(state.back_ref, state.sym - 256);
            new.extend(self.completions_tx.query(state.back_ref, state.sym));
            return;
        };
        if sym < 256 {
            // Direct matches on the input symbol advance the state,
            // otherwise this branch fails to parse and we drop the state
            if self.input_symbol == sym as u8 {
                self.next_states
                    .push(mk_state(state.back_ref, state.sym, &state.remaining[1..]));
            }
        } else {
            // To match a nonterminal, expand all the rules for it,
            // and remember our state as a completion if the nonterminal successfully
            // parses.
            self.completions_tx.push((
                sym,
                mk_state(state.back_ref, state.sym, &state.remaining[1..]),
            ));
            new.extend(
                self.cfg
                    .rules_for(sym)
                    .map(|r| mk_state(self.completions_tx.batch_id(), sym, &r.parts[..])),
            );
        }
    }
}
