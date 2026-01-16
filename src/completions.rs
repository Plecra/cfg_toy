use crate::CfgSymbol;

use super::recognizer::{NtSymbol, State};

// pub(crate) type Completion<'a, Symbol> = (NtSymbol, State<'a, Symbol>);
pub(crate) type Completion<'a, Symbol> = (NtSymbol, (usize, NtSymbol, Remaining<'a, Symbol>));
#[derive(Debug)]
pub enum Remaining<'a, Symbol> {
    EmptyAndForwardingTo(usize, usize),
    More(&'a [Symbol]),
}
impl<Symbol> Clone for Remaining<'_, Symbol> {
    fn clone(&self) -> Self {
        match self {
            Remaining::EmptyAndForwardingTo(a, b) => Remaining::EmptyAndForwardingTo(*a, *b),
            Remaining::More(syms) => Remaining::More(syms),
        }
    }
}
/// Semantically, this is a `BTreeMap<(usize, NtSymbol), State<'a>>`
/// It's implemented via a flat buffer containing all the entries in correct order,
/// and the completion index for locating each value of `usize`. This
/// provides range queries for `(i, sym)` efficiently.
#[derive(Debug)]
pub struct Completions<'a, Symbol> {
    // Invariant: all of the forwarding records have nonempty remaining symbols.
    // These are used to build cache entries
    pub forwarding_records: Vec<State<'a, Symbol>>,
    pub completions: Vec<Completion<'a, Symbol>>,
    pub completion_index: Vec<usize>,
}
impl<'a, Symbol: CfgSymbol> Completions<'a, Symbol> {
    pub(crate) fn new(len: usize) -> Self {
        let completions = vec![];
        let mut completion_index = Vec::with_capacity(len + 1);
        completion_index.push(0);
        Self {
            forwarding_records: vec![],
            completions,
            completion_index,
        }
    }
    fn query_range(&self, back_ref: usize, sym: NtSymbol) -> std::ops::Range<usize> {
        let start = self.completion_index[back_ref];
        let end = self.completion_index[back_ref + 1];
        let start_of_comps = start + self.completions[start..end].partition_point(|c| c.0 < sym);
        let end_of_comps =
            start_of_comps + self.completions[start_of_comps..end].partition_point(|c| c.0 <= sym);
        start_of_comps..end_of_comps
    }
    pub(crate) fn query<'b>(
        &'b mut self,
        back_ref: usize,
        sym: NtSymbol,
    ) -> impl Iterator<Item = State<'a, Symbol>> + 'b {
        let mut range = self.query_range(back_ref, sym);
        // let completions = &mut self.completions;
        // let forwarding_records: &'b [_] = &self.forwarding_records;
        let mut forwarding_drain = 0..0;
        core::iter::from_fn(move ||  {
            loop {
                println!("Querying {back_ref} {sym} range={:?} {:?} fwd={:?}", range, &self.completions[range.clone()], forwarding_drain);
                if let Some(i) = forwarding_drain.next() {
                    return Some(self.forwarding_records[i].clone());
                }
                let i = range.next()?;
                let (back_ref, sym, rem) = self.completions[i].1.clone();
                break Some(match rem {
                    // FIXME?: there is a case where we find a forwarding candidate, but it's wasteful.
                    // in particular,
                    // alpha ::= "a".
                    // alpha ::= "b".
                    // other_rule ::= alpha.
                    // when we recognize the alpha, the other_rule will have a completion of the form
                    // (alpha, (other_rule, idx, []))
                    // we would normally allocate a bypass to switch this to 
                    // (alpha, (other_rule, idx, bypass([..states_after_other_rule])))
                    // however, no other rules on alpha could possibly match at the same position:
                    // "a" isn't the prefix of any other rule for `alpha`.
                    // so this bypass is statically unnecessary.
                    // This also applies for the `ident` rule, because
                    // we can statically know that all completions that hit an ident will have hit
                    // an `alpha`, so we should just store the bypass for the alpha.
                    // 
                    // I think this requires performing this analysis on the cfg ahead of time,
                    // this property doesnt seem to be available locally. if another rule was
                    // `alpha ::= "a" "a".`, then it'd get use out of the bypass.
                    //
                    // There's also the question of how *much* use is worth paying for allocating
                    // the bypass. again, a static analysis is probably helpful here, we can
                    // count the number of potential reuses via different rules.
                    // each rule can know "can occur as prefix of sibling rules > N times" as a flag.
                    Remaining::More([])  => {
                        ;
                        fn setup_bypass<'a, Symbol: CfgSymbol>(
                            completions: &mut Completions<'a, Symbol>,
                            empty_rem_i: usize,
                            back_ref: usize,
                            sym: NtSymbol,
                        ) -> std::ops::Range<usize> {
                            let forward_to = completions.query_range(back_ref, sym);
                            for j in forward_to.clone() {
                                let (b_ref, s, rem) = completions.completions[j].1.clone();
                                let Remaining::More([]) = rem else { continue };
                                setup_bypass(completions, j, b_ref, s);
                            }
                            if forward_to.len() == 1 &&
                                let Remaining::EmptyAndForwardingTo(start, end ) = completions.completions[forward_to.start].1.2
                            {
                                // already set up
                                completions.completions[empty_rem_i].1.2 =
                                    Remaining::EmptyAndForwardingTo(start, end);
                                start..end
                            } else {

                                let start = completions.forwarding_records.len();
                                for j in forward_to {
                                    let (b_ref, s, rem) = completions.completions[j].1.clone();
                                    match rem {
                                        Remaining::More(syms) => {
                                            completions.forwarding_records.push(State {
                                                back_ref: b_ref,
                                                sym: s,
                                                remaining: syms,
                                            });
                                        }
                                        Remaining::EmptyAndForwardingTo(st, end) => {
                                            completions.forwarding_records
                                                .extend_from_within(st..end);
                                        }
                                    }
                                }
                                let end = completions.forwarding_records.len();
                                println!("{back_ref} {sym} {:?}", &completions.forwarding_records[start..end]);
                                if start != end {
                                    completions.completions[empty_rem_i].1.2 =
                                        Remaining::EmptyAndForwardingTo(start, end);
                                }
                                start..end
                            }
                        }
                        forwarding_drain = setup_bypass(self, i, back_ref, sym);
                        continue;
                        // if forward_to.len() == 1 {

                        //     (self.completion_index[i].1).2 = todo!();
                        // }
                    }
                    Remaining::More(syms) => State {
                        back_ref,
                        sym,
                        remaining: syms,
                    },
                    Remaining::EmptyAndForwardingTo(start, end) => {
                        forwarding_drain = start..end;
                        continue;
                    }
                })
            }
        })
        // self.completions[range]
        //     .iter_mut()
        //     .flat_map(|c| {
        //         let (back_ref, sym, rem) = c.1.clone();
        //     })
    }
    pub(crate) fn query_without_cache_update(
        &self,
        back_ref: usize,
        sym: NtSymbol,
    ) -> impl Iterator<Item = State<'a, Symbol>> + '_ {
        let range = self.query_range(back_ref, sym);
        self.completions[range]
            .iter()
            .flat_map(|c| {
                let (back_ref, sym, rem) = c.1.clone();
                match rem {
                    Remaining::More(syms) => Either::Left(core::iter::once(State {
                        back_ref,
                        sym,
                        remaining: syms,
                    })),
                    Remaining::EmptyAndForwardingTo(start, end) => {
                        Either::Right(self.forwarding_records[start..end].iter().cloned())
                    }
                }
            })
    }
}

enum Either<A, B> {
    Left(A),
    Right(B),
}
impl<A, B> Iterator for Either<A, B>
where
    A: Iterator,
    B: Iterator<Item = A::Item>,
{
    type Item = A::Item;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Either::Left(a) => a.next(),
            Either::Right(b) => b.next(),
        }
    }
}

impl<'a, Symbol: Ord + CfgSymbol> Completions<'a, Symbol> {
    pub(crate) fn add_group(&mut self) -> CompletionsTransaction<'a, '_, Symbol> {
        CompletionsTransaction::new(self)
    }
}
// Appending a new group to the Completions buffer needs to be careful to
// clean up once it's done, which this type is responsible for.
pub(crate) struct CompletionsTransaction<'a, 'b, Symbol: Ord> {
    completions: &'b mut Completions<'a, Symbol>,
    start_len: usize,
}
impl<'a, 'b, Symbol: Ord + CfgSymbol> CompletionsTransaction<'a, 'b, Symbol> {
    fn new(completions: &'b mut Completions<'a, Symbol>) -> Self {
        let start_len = completions.completions.len();
        Self {
            completions,
            start_len,
        }
    }
    pub(crate) fn query(
        &mut self,
        back_ref: usize,
        sym: NtSymbol,
    ) -> impl Iterator<Item = State<'a, Symbol>> + '_ {
        self.completions.query(back_ref, sym)
    }
    pub(crate) fn push(&mut self, nt: NtSymbol, state: State<'a, Symbol>) {
        self.completions.completions.push((
            nt,
            (state.back_ref, state.sym, Remaining::More(state.remaining)),
        ));
    }
    pub(crate) fn batch_id(&self) -> usize {
        self.completions.completion_index.len() - 1
    }
}
impl<'a, 'b, Symbol: Ord> Drop for CompletionsTransaction<'a, 'b, Symbol> {
    fn drop(&mut self) {
        self.completions.completions[self.start_len..].sort_by_key(|c| c.0);
        self.completions
            .completion_index
            .push(self.completions.completions.len());
    }
}
