use std::borrow::Borrow;

use crate::buffer_pair::{BufferPair, Transfer};
use crate::completions::{Completions, CompletionsTransaction};
use crate::set_buffers::{grow_ordered_set, isolate_new_elements, sorted_set};

pub trait TraceAt<'a, Symbol> {
    fn completed(&mut self, back_ref: usize, sym: NtSymbol, rule: &'a [Symbol]);
}
pub trait Trace<'a, Symbol> {
    fn at(&mut self, symbol_index: usize) -> impl TraceAt<'a, Symbol> + '_;
}
impl<'a, T: Trace<'a, Symbol>, Symbol> Trace<'a, Symbol> for &'_ mut T {
    fn at(&mut self, symbol_index: usize) -> impl TraceAt<'a, Symbol> + '_ {
        (**self).at(symbol_index)
    }
}
impl<'a, Symbol> Trace<'a, Symbol> for () {
    fn at(&mut self, _symbol_index: usize) -> impl TraceAt<'a, Symbol> + '_ {}
}
impl<'a, Symbol> TraceAt<'a, Symbol> for () {
    fn completed(&mut self, _back_ref: usize, _sym: NtSymbol, _rule: &'a [Symbol]) {}
}

// type Symbol = u32;
pub(crate) type NtSymbol = u32;
// TODO: can switch this to encoding the sym id into the slice.
// the standard presentation is that these store (Rule, rule_offset)

#[derive(Copy, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct State<'a, Symbol> {
    pub back_ref: usize,
    pub sym: NtSymbol,
    pub rule: &'a [Symbol],
    pub remaining: &'a [Symbol],
}
impl<'a, Symbol> Clone for State<'a, Symbol> {
    fn clone(&self) -> Self {
        Self {
            back_ref: self.back_ref,
            sym: self.sym,
            rule: self.rule,
            remaining: self.remaining,
        }
    }
}
fn mk_state<'a, Symbol>(back_ref: usize, sym: NtSymbol, rule: &'a [Symbol], remaining: &'a [Symbol]) -> State<'a, Symbol> {
    State {
        back_ref,
        sym,
        rule,
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
    mut trace: impl Trace<'c, Symbol>,
) -> Completions<'c, Symbol> {
    // println!("initial states: {:?}", states);
    // This is kept between iterations for double buffering to
    // save on allocating it.
    let mut next_states = vec![];

    // TODO: The completions should sorta have a GC pass. Especially for longer files,
    // most of its content is completely unreferenced.
    // Currently using binary search maps due to the need for range queries, it'd be totally sensible
    // to revisit that
    let mut completions = Completions::new(src.len());

    let mut states = cfg
        .query_nt(init_sym)
        .unwrap()
        // .filter(|i| !cfg.rule_nullable[*i])
        .map(|i| &cfg.rules[i])
        .filter(|r| !r.parts.is_empty())
        .map(|r| mk_state(0, init_sym, &r.parts[..], &r.parts[..]))
        .collect::<Vec<_>>();
    // println!("wut {:?}", cfg.query_nullable(init_sym).unwrap());
    // println!("wut {:?}", cfg
    //     .query_nt(init_sym)
    //     .unwrap()
    //     // .filter(|i| !cfg.rule_nullable[*i])
    //     .map(|i| &cfg.rules[i]).collect::<Vec<_>>());
    // println!("{cfg:?}");
    for &rule in &cfg.nt_to_nullable_rules_index[cfg.query_nullable(init_sym).unwrap()] {
        trace.at(0).completed(0, init_sym, &cfg.rules[rule].parts);
    }

    for (cursor, input_symbol) in src.iter().enumerate() {
        // println!("{cursor}@{states:?}");
        let mut step = EarleyStep {
            cfg,
            input_symbol,
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
        // TODO: we can improve the representation of states to encode the rule + nonterminal much more efficiently:
        // The remaining data should be stored as an offset into the "rule data", where it's held as a null terminated
        // string. These offsets can then use the high bits to encode the nonterminal and a rule id.
        // These rule ids can be prepared ahead of time to deduplicate identical suffixes, so that A ::= B C. and A :: C D.
        // would be represented by offset = rule_1_idx and offset = rule_2_idx, which would differ only in the bitmask
        // for the rule they're from, so the deduplication step can merge them to the version of the rule that completes
        // as rule_1 *and* rule_2.
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
                trace.at(src.len()).completed(state.back_ref, state.sym, state.rule);
                // println!("completed state report: {:?}", state);
                states
                    .write()
                    .extend(completions_tx.query(state.back_ref, state.sym)
                    // .inspect(|c| println!("have completion {c:?}"))
                );
                continue;
            } else {
                let sym = state.remaining.first().unwrap();
                match sym.as_part() {
                    super::Either::Ok(_) => (),
                    super::Either::Err(nt) => {
                        // Synthesize a completion that'll never be used,
                        // we still need to indicate that ws is a valid child for us
                        completions_tx.push(
                            nt,
                            mk_state(state.back_ref, state.sym,
                                state.rule, &state.remaining[1..]),
                        );
                        // FIXME: transitive please
                        let can_skip = cfg.rules_for(nt).any(|rule| rule.parts.is_empty());
                        if can_skip {
                            trace.at(src.len()).completed(src.len(), nt, &[]);
                            states.write().push(mk_state(
                                state.back_ref,
                                state.sym,
                                state.rule,
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

struct PrintRemainingList<'a, Symbol>(
    &'a [(u32, (usize, u32, &'a [Symbol], crate::completions::Remaining<'a, Symbol>))],
    &'a [crate::recognizer::State<'a, Symbol>],
);
impl<Symbol: core::fmt::Debug> core::fmt::Debug for PrintRemainingList<'_, Symbol> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for &(ntsym, (back_ref, sym_here, rule, ref rem)) in self.0 {
            match rem {
                crate::completions::Remaining::EmptyAndForwardingTo(start, end) => {
                    list.entry(&format_args!(
                        "\n  ({}, ({}, {}, {:?}, bypass[{}..{}]({:?})))",
                        ntsym,
                        back_ref,
                        sym_here,
                        rule,
                        start,
                        end,
                        &self.1[*start..*end]
                    ));
                }
                crate::completions::Remaining::More(syms) => {
                    list.entry(&format_args!("\n  {:?}", (ntsym, (back_ref, sym_here, rule, syms))));
                }
            }
        }
        list.finish()
    }
}
impl<'c, T: TraceAt<'c, Symbol>, Symbol: super::CfgSymbol + Ord> EarleyStep<'c, '_, T, Symbol> {
    fn expand_states(&mut self, mut transfer: impl BufferPair<State<'c, Symbol>>) {
        for i in 0..transfer.read().len() {
            let state = transfer.read()[i].clone();
            self.expand_state(state, transfer.write());
        }
    }
    fn expand_state(&mut self, state: State<'c, Symbol>, new: &mut Vec<State<'c, Symbol>>) {
        let Some(sym) = state.remaining.first() else {
            // This state has recognized its nontermininal starting at state.back_ref
            self.trace.completed(state.back_ref, state.sym, state.rule);
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
                        state.rule,
                        &state.remaining[1..],
                    ));
                }
            }
            super::Either::Err(nt) => {
                // println!("predicting for sym {sym:?}");
                // To match a nonterminal, expand all the rules for it,
                // and remember our state as a completion if the nonterminal successfully
                // parses.
                self.completions_tx.push(
                    nt,
                    mk_state(state.back_ref, state.sym, 
                                state.rule,&state.remaining[1..]),
                );

                // We are about to predict a nonterminal.
                // When an eta rule exists for it, it would attempt to dereference a back_ref
                // that isnt live yet, therefore we must only generate states into `new`
                // with at least length 1.
                //
                // There's a further wrinkle though: We *do* need to generate nonterminals
                // for the rules that we're trying to match. So they'll still run into the
                // case where they're dereferencing a back_ref that isn't live yet.
                // To handle that, we need to eagerly expand nullable nonterminals here.
                // Then the later expansions can be skipped and rely on already being
                // performed further up.

                if self.cfg.nt_nullable[nt as usize] {
                    // If the nonterminal is nullable, we can also skip it directly
                    #[allow(clippy::never_loop)]
                    for &rule in &self.cfg.nt_to_nullable_rules_index[self.cfg.query_nullable(nt).unwrap()] {
                        // println!("completing nullable rule {rule:?} {:?} for NT {}", self.cfg.rules[rule], nt);
                        // println!("{:?}", &self.cfg.nt_to_nullable_rules_index[self.cfg.query_nullable(nt).unwrap()]);
                        // println!("{:?}", &self.cfg.nt_to_nullable_rules_index[self.cfg.query_nullable(nt).unwrap()].iter().map(|&i| &self.cfg.rules[i]).collect::<Vec<_>>());
                        // println!("{:?}", &self.cfg.nt_to_nullable_rules_index[self.cfg.query_nullable(nt + 1).unwrap()].iter().map(|&i| &self.cfg.rules[i]).collect::<Vec<_>>());
                        // panic!();
                        self.trace.completed(self.completions_tx.batch_id(), nt, &self.cfg.rules[rule].parts);
                    }
                    // let mut visited = std::collections::HashSet::new();
                    // visited.insert(nt);
                    // let mut todo = vec![nt];
                    // while let Some(nt) = todo.pop() {
                    //     for &idx in &self.cfg.nt_to_nullable_rules_index[self.cfg.query_nullable(nt).unwrap()]
                    //     {
                    //         let parts = &self.cfg.rules[idx].parts[..];
                    //         for (i, part) in parts.iter().enumerate() {
                    //             let child_symbol = match part.as_part() {
                    //                 Ok(_) => unreachable!(),
                    //                 Err(nnt) => nnt,
                    //             };
                    //             if visited.contains(&child_symbol) {
                    //                 continue;
                    //             }
                    //             self.completions_tx.push((
                    //                 nt,
                    //                 mk_state(self.completions_tx.batch_id(), child_symbol, &parts[i..]),
                    //             ));

                    //             self.trace.completed(self.completions_tx.batch_id(), child_symbol);
                    //             visited.insert(child_symbol);
                    //             todo.push(child_symbol);
                    //         }
                    //     }
                    // }
                    if state.remaining.len() != 1 || state.back_ref < self.completions_tx.batch_id()
                    {
                        self.expand_state(
                            mk_state(state.back_ref, state.sym,
                                state.rule, &state.remaining[1..]),
                            new,
                        );
                    }
                }

                for rule in self.cfg.rules_for(nt) {
                    // FIXME: This also needs to be done transitively:
                    //
                    if rule.parts.is_empty() {
                        //     // println!("    (epsilon)");
                        //     // FIXME: Empty matches can be duplicated in the trace,
                        //     // and should likely be de-duplicated.
                        //     // This is still the most straightforward implementation for
                        //     // the empty rules since it means the order that they're
                        //     // processed in can't matter.
                        //     // afaict it would also be possible to deliberately sort
                        //     // out the empty rules and fire them after everything else
                        //     // *then* repeat that until no more empty rules are left.
                        //     self.trace.completed(self.completions_tx.batch_id(), nt);
                        //     self.expand_state(
                        //         mk_state(state.back_ref, state.sym, &state.remaining[1..]),
                        //         new,
                        //     );
                    } else {
                        new.push(mk_state(
                            self.completions_tx.batch_id(),
                            nt,
                            &rule.parts[..],
                            &rule.parts[..],
                        ));
                    }
                }
            }
        }
    }
}
