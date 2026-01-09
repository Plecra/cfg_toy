mod completions;
use completions::{Completions, CompletionsTransaction};
#[derive(Debug)]
struct Rule {
    for_nt: u32,
    parts: Vec<u32>,
}
#[derive(Debug)]
struct Cfg {
    rules: Vec<Rule>,
}
impl Cfg {
    fn rules_for(&self, nt: u32) -> impl Iterator<Item = &'_ Rule> + '_ {
        let start = self.rules.partition_point(|r| r.for_nt < nt);
        let end = self.rules.partition_point(|r| r.for_nt <= nt);
        // TODO: bench? this is the same as iterating while we're in the group
        // self
        //     .rules[start..].iter()
        //     .take_while(|r| r.for_nt == nt)
        self.rules[start..end].iter()
    }
}
macro_rules! cfg_rules {
    {$cx:ident $rule_name:ident $($t:tt)*} => {
        $cx.1.push($rule_name);
        cfg_rules!($cx $($t)*)
    };
    {$cx:ident $literal:literal $($t:tt)*} => {
        $cx.1.extend($literal.as_bytes().iter().map(|&b| b as u32));
        cfg_rules!($cx $($t)*);
    };
    {$cx:ident . $rulename:ident :: = $($t:tt)*} => {
        $cx.0.push(Rule {
            parts: std::mem::take(&mut $cx.1),
            for_nt: $cx.2,
        });
        $cx.2 = $rulename;
        cfg_rules!($cx $($t)*);
    };
    {$cx:ident .} => {
        $cx.0.push(Rule {
            parts: std::mem::take(&mut $cx.1),
            for_nt: $cx.2,
        });
    };
}
macro_rules! cfg {
    {
        $($states:ident)*;
        $first_rule:ident ::= $($rule_definition:tt)*
    } => {{
        let mut states = 256u32;
        $(let $states = states; #[allow(unused_assignments)] { states += 1; })*
        let mut cx: (Vec<Rule>, Vec<u32>, u32) = (vec![], vec![], $first_rule);
        cfg_rules!(cx $($rule_definition)*);
        Cfg { rules: cx.0 }
    }};
}
struct Node {
    // transition: u32,
    // start: usize,
    // end: usize,
    // parent: usize,
    // // next_sibling: usize,
}
type Ast = Vec<Node>;
// type Symbol = u32;
type NtSymbol = u32;
// TODO: can switch this to encoding the sym id into the slice.
// the standard presentation is that these store (Rule, rule_offset)
type Completion<'a> = (NtSymbol, State<'a>);
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
struct State<'a> {
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

// Here's some unfortunate complexity, this boilerplate is just responsible
// for allowing us to read + write to the same reference.
trait StateGrouping<T> {
    fn read(&self) -> &[T];
    fn write(&mut self) -> &mut Vec<T>;
}
impl<'b> StateGrouping<State<'b>> for &'_ mut Vec<State<'b>> {
    fn read(&self) -> &[State<'b>] {
        self
    }
    fn write(&mut self) -> &mut Vec<State<'b>> {
        self
    }
}
impl<'b> StateGrouping<State<'b>> for Vec<State<'b>> {
    fn read(&self) -> &[State<'b>] {
        self
    }
    fn write(&mut self) -> &mut Vec<State<'b>> {
        self
    }
}
struct FromOldStates<'a, 'c> {
    states: &'a Vec<State<'c>>,
    new_states: Vec<State<'c>>,
}
impl<'a, 'b> StateGrouping<State<'b>> for FromOldStates<'a, 'b> {
    fn read(&self) -> &[State<'b>] {
        self.states
    }
    fn write(&mut self) -> &mut Vec<State<'b>> {
        &mut self.new_states
    }
}
struct InternalSlice<'a, T> {
    slice: &'a mut Vec<T>,
    range: std::ops::Range<usize>,
}
impl<'a, T> StateGrouping<T> for InternalSlice<'a, T> {
    fn read(&self) -> &[T] {
        &self.slice[self.range.clone()]
    }
    fn write(&mut self) -> &mut Vec<T> {
        self.slice
    }
}


fn parse_earley(cfg: &Cfg, src: &[u8], init_sym: u32) -> Ast {
    let mut states = cfg
        .rules_for(init_sym)
        .map(|r| mk_state(0, init_sym, &r.parts[..]))
        .collect::<Vec<_>>();

    // TODO: The completions should sorta have a GC pass. Especially for longer files,
    // most of its content is completely unreferenced.
    // Currently using binary search maps due to the need for range queries, it'd be totally sensible
    // to revisit that
    let mut completions = Completions::new(src.len());

    for cursor in 0..src.len() {
        // As we expand the states, we'll generate more states that need to be processed.
        // we keep track of all generated states here to deduplicate them
        let mut transfer = FromOldStates {
            states: &states,
            new_states: vec![],
        };
        // The states for the next character get accumulated here, they'll need to be deduplicated
        // before we actually process the next character
        let mut next_states = vec![];
        // If any state transition is a prediction, we remember the completion for it to use later
        let mut completions_tx = completions.add_group();
        // To optimize deduplicating the new states, we deduplicate in batches, so that nothing
        // before the current pass needs to be checked again.
        let states_before_pass = transfer.new_states.len();
        
        expand_states(&mut transfer, &mut next_states, &mut completions_tx, cfg, cursor, src);
        let mut new_states = transfer.new_states;

        isolate_new_elements(&mut new_states, states_before_pass);
        grow_ordered_set(&mut new_states, |mut states| {
            expand_states(&mut states, &mut next_states, &mut completions_tx, cfg, cursor, src);
        });
        sorted_set(&mut next_states);
        states = next_states;
    }
    states.sort();
    grow_ordered_set(&mut states, |mut states| {
        for i in 0..states.read().len() {
            let state = states.read()[i];
            states.write().extend(completions.query(state.back_ref, state.sym));
        }
    });
    // the match state is (back_ref: 0, sym: 256), so will always be at the start
    println!("{:?}", states[0]);

    vec![]
}
// Find the transitive closure of a relation
fn grow_ordered_set<T: Ord + Clone>(
    states: &mut Vec<T>,
    mut rel: impl FnMut(InternalSlice<'_, T>),
) {
    let mut loop_check = {
        let mut iters = 0;
        move || {
            iters += 1;
            if 120 <= iters {
                panic!("recursion limit?");
            }
        }
    };
    let mut pending_start = 0;
    while pending_start < states.len() {
        loop_check();
        let pending_end = states.len();
        rel(InternalSlice { slice: states, range: pending_start..pending_end });
        states[..pending_end].sort();
        isolate_new_elements(states, pending_end);
        pending_start = pending_end;
    }
}
fn isolate_new_elements<T: Ord>(states: &mut Vec<T>, old_len: usize) {
    let (old, new) = states.split_at_mut(old_len);
    new.sort();
    let mut check = 0;
    let new_len = slice_retain_with_context(new, |cx, new_val| {
        while check < old.len() && old[check] < *new_val {
            check += 1;
        }
        cx.last() != Some(new_val) && (check == old.len() || old[check] != *new_val)
    });
    states.truncate(old_len + new_len);
}
fn expand_states<'c>(
    transfer: &mut impl StateGrouping<State<'c>>,
    next_states: &mut Vec<State<'c>>,
    completions: &mut CompletionsTransaction<'c, '_>,
    cfg: &'c Cfg,
    cursor: usize,
    src: &[u8],
) {
    for i in 0..transfer.read().len() {
        let state = transfer.read()[i];
        expand_states_(state, transfer, completions, src, cursor, next_states, cfg);
    }
}
fn expand_states_<'c>(
    state: State<'c>, 
    transfer: &mut impl StateGrouping<State<'c>>,
    completions: &mut CompletionsTransaction<'c, '_>,
    src: &[u8],
    cursor: usize,
    next_states: &mut Vec<State<'c>>,
    cfg: &'c Cfg,) {

    let Some(&sym) = state.remaining.first() else {
        let new = transfer.write();
        new.extend(completions.query(state.back_ref, state.sym));
        return;
    };
    if sym < 256 {
        if src[cursor] == sym as u8 {
            next_states.push(mk_state(state.back_ref, state.sym, &state.remaining[1..]));
        }
    } else {
        completions.push((
            sym,
            mk_state(state.back_ref, state.sym, &state.remaining[1..]),
        ));
        transfer.write().extend(
            cfg.rules_for(sym)
                .map(|r| mk_state(cursor, sym, &r.parts[..])),
        );
    }
}
fn vec_dedup<T: PartialEq>(vec: &mut Vec<T>) {
    retain_with_context(vec, |cx, v| cx.last() != Some(v));
}
fn sorted_set<T: PartialEq + Ord>(vec: &mut Vec<T>) {
    vec.sort();
    vec_dedup(vec);
}
fn slice_retain_with_context<T, F>(vec: &mut [T], mut f: F) -> usize
where
    F: FnMut(&mut [T], &mut T) -> bool,
{
    let mut write = 0;
    for read in 0..vec.len() {
        let (retained, tail) = vec.split_at_mut(write);
        let current = &mut tail[read - write];
        if f(retained, current) {
            vec.swap(write, read);
            write += 1;
        }
    }
    write
}
fn retain_with_context<T, F>(vec: &mut Vec<T>, f: F)
where
    F: FnMut(&mut [T], &mut T) -> bool,
{
    let len = slice_retain_with_context(&mut vec[..], f);
    vec.truncate(len);
}
fn main() {
    let mut mycfg = cfg! {
        expr and_expr primary alpha ident ws gap and or not;

        ws ::= " " .
        ws ::= "\n" .
        gap ::= ws.
        gap ::= ws gap.

        alpha ::= "a".
        alpha ::= "b".
        alpha ::= "c".
        ident ::= alpha ident.
        ident ::= alpha.

        and ::= gap "and" gap.
        or ::= gap "or" gap.
        not ::= "not" gap.

        expr ::= and_expr or expr.
        expr ::= and_expr.
        and_expr ::= primary and and_expr.
        and_expr ::= primary.
        primary ::= not primary.
        primary ::= ident.
        primary ::= "(" expr ")".
        primary ::= "true".
        primary ::= "false".
    };
    println!("{:?}", mycfg);
    mycfg.rules.sort_by_key(|rule| rule.for_nt);
    let _ = parse_earley(&mycfg, "true or false and b  and not true".as_bytes(), 256);
}
