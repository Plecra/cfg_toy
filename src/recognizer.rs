use core::sync;
use std::borrow::Borrow;

use crate::buffer_pair::{BufferPair, Transfer};
use crate::completions::{Completions, CompletionsTransaction};
use crate::set_buffers::{grow_ordered_set, isolate_new_elements, sorted_set};

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
pub(crate) type Completion<'a, Symbol> = (NtSymbol, State<'a, Symbol>);
#[derive(Copy, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) struct State<'a, Symbol> {
    pub(crate) back_ref: usize,
    pub(crate) sym: NtSymbol,
    pub(crate) remaining: &'a [Symbol],
}
impl<'a, Symbol> Clone for State<'a, Symbol> {
    fn clone(&self) -> Self {
        Self {
            back_ref: self.back_ref,
            sym: self.sym,
            remaining: self.remaining,
        }
    }
}
fn mk_state<Symbol>(back_ref: usize, sym: NtSymbol, remaining: &[Symbol]) -> State<'_, Symbol> {
    State {
        back_ref,
        sym,
        remaining,
    }
}

struct EarleyStep<'c, 'r, T, Symbol: Ord + super::CfgSymbol> {
    cfg: &'c crate::grammar::Cfg<Symbol>,
    // TODO(opts): Try making this `u8` instead of `&u8` while parsing a normal buffer
    input_symbol: &'c Symbol::Terminal,
    completions_tx: CompletionsTransaction<'c, 'r, Symbol>,
    next_states: Vec<State<'c, Symbol>>,
    trace: T,
}
pub fn parse_earley<'c, Symbol: super::CfgSymbol + Ord>(
    cfg: &'c crate::grammar::Cfg<Symbol>,
    src: &'c [Symbol::Terminal],
    init_sym: u32,
    mut trace: impl Trace,
) -> Completions<'c, Symbol> {
    let mut states = cfg
        .rules_for(init_sym)
        .map(|r| mk_state(0, init_sym, &r.parts[..]))
        .collect::<Vec<_>>();
    println!("initial states: {:?}", states);
    // This is kept between iterations for double buffering to
    // save on allocating it.
    let mut next_states = vec![];

    // TODO: The completions should sorta have a GC pass. Especially for longer files,
    // most of its content is completely unreferenced.
    // Currently using binary search maps due to the need for range queries, it'd be totally sensible
    // to revisit that
    let mut completions = Completions::new(src.len());

    for cursor in 0..src.len() {
        println!("{cursor}@{states:?}");
        let mut step = EarleyStep {
            cfg,
            input_symbol: &src[cursor],
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
            println!("{:?}", states.read());
            step.expand_states(states);
        });
        sorted_set(&mut step.next_states);

        let mut used_up_states = std::mem::replace(&mut states, step.next_states);
        used_up_states.clear();
        next_states = used_up_states;
        if states.is_empty() {
            panic!("no states left at cursor {}", cursor);
        }
    }
    let mut completions_tx = completions.add_group();
    grow_ordered_set(&mut states, |mut states| {
        for i in 0..states.read().len() {
            let state = states.read()[i].clone();
            if state.remaining.is_empty() {
                // This state has recognized its nontermininal starting at state.back_ref
                trace.at(src.len()).completed(state.back_ref, state.sym);
                states
                    .write()
                    .extend(completions_tx.query(state.back_ref, state.sym));
                continue;
            } else {
                let sym = state.remaining.first().unwrap();
                match sym.as_part() {
                    super::Either::Ok(_) => (),
                    super::Either::Err(nt) => {
                        // Synthesize a completion that'll never be used,
                        // we still need to indicate that ws is a valid child for us
                        completions_tx.push((
                            nt,
                            mk_state(state.back_ref, state.sym, &state.remaining[1..]),
                        ));
                        // FIXME: transitive please
                        let can_skip = cfg.rules_for(nt).any(|rule| dbg!(rule).parts.is_empty());
                        if can_skip {
                            trace.at(src.len()).completed(src.len(), nt);
                            states.write().push(mk_state(
                                state.back_ref,
                                state.sym,
                                &state.remaining[1..],
                            ))
                        }
                    }
                }
            }
        }
    });
    drop(completions_tx);
    // the match state is (back_ref: 0, sym: 256), so will always be at the start
    // println!("final states: {:?}", states);
    assert_eq!(
        states.first().map(|s| (s.back_ref, s.sym)),
        Some((0, init_sym))
    );
    completions
}
impl<'c, T: TraceAt, Symbol: super::CfgSymbol + Ord> EarleyStep<'c, '_, T, Symbol> {
    fn expand_states(&mut self, mut transfer: impl BufferPair<State<'c, Symbol>>) {
        for i in 0..transfer.read().len() {
            let state = transfer.read()[i].clone();
            self.expand_state(state, transfer.write());
        }
    }
    fn expand_state(&mut self, state: State<'c, Symbol>, new: &mut Vec<State<'c, Symbol>>) {
        let Some(sym) = state.remaining.first() else {
            // This state has recognized its nontermininal starting at state.back_ref
            println!("done with {:?}", state);
            self.trace.completed(state.back_ref, state.sym);
            new.extend(self.completions_tx.query(state.back_ref, state.sym));
            return;
        };
        match sym.as_part() {
            super::Either::Ok(sym) => {
                // println!("trying to match sym {:?} == {:?}", self.input_symbol, *sym.borrow());
                // Direct matches on the input symbol advance the state,
                // otherwise this branch fails to parse and we drop the state
                if self.input_symbol == sym.borrow() {
                    // println!("matches {:?}", *sym.borrow());

                    self.next_states.push(mk_state(
                        state.back_ref,
                        state.sym,
                        &state.remaining[1..],
                    ));
                }
            }
            super::Either::Err(nt) => {
                // println!("predicting for sym {sym:?}");
                // To match a nonterminal, expand all the rules for it,
                // and remember our state as a completion if the nonterminal successfully
                // parses.
                self.completions_tx.push((
                    nt,
                    mk_state(state.back_ref, state.sym, &state.remaining[1..]),
                ));
                
                for rule in self.cfg.rules_for(nt) {
                    // FIXME: This also needs to be done transitively:
                    // 
                    if rule.parts.is_empty() {
                        // println!("    (epsilon)");
                        // FIXME: Empty matches can be duplicated in the trace,
                        // and should likely be de-duplicated.
                        // This is still the most straightforward implementation for
                        // the empty rules since it means the order that they're
                        // processed in can't matter.
                        // afaict it would also be possible to deliberately sort
                        // out the empty rules and fire them after everything else
                        // *then* repeat that until no more empty rules are left.
                        self.trace.completed(self.completions_tx.batch_id(), nt);
                        self.expand_state(mk_state(
                            state.back_ref,
                            state.sym,
                            &state.remaining[1..],
                        ), new);
                    } else {
                        new.push(mk_state(
                            self.completions_tx.batch_id(),
                            nt,
                            &rule.parts[..],
                        ));
                    }
                }
            }
        }
    }
}
