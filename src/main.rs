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
            parts: std::mem::replace(&mut $cx.1, Default::default()),
            for_nt: $cx.2,
        });
        $cx.2 = $rulename;
        cfg_rules!($cx $($t)*);
    };
    {$cx:ident .} => {
        $cx.0.push(Rule {
            parts: std::mem::replace(&mut $cx.1, Default::default()),
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
        $(let $states = states; states += 1;)*
        let mut cx: (Vec<Rule>, Vec<u32>, u32) = (vec![], vec![], $first_rule);
        cfg_rules!(cx $($rule_definition)*);
        Cfg { rules: cx.0 }
    }};
}
struct Node {
    transition: u32,
    start: usize,
    end: usize,
    parent: usize,
    // next_sibling: usize,
}
type Ast = Vec<Node>;
type Symbol = u32;
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
fn State(back_ref: usize, sym: NtSymbol, remaining: &[u32]) -> State<'_> {
    State {
        back_ref,
        sym,
        remaining,
    }
}

// Here's some unfortunate complexity, this boilerplate is just responsible
// for allowing us to read + write to the same reference.
trait StateGrouping<'c> {
    fn read(&self) -> &Vec<State<'c>>;
    fn write(&mut self) -> &mut Vec<State<'c>>;
}
impl<'a, 'b> StateGrouping<'b> for &'a mut Vec<State<'b>> {
    fn read(&self) -> &Vec<State<'b>> {
        self
    }
    fn write(&mut self) -> &mut Vec<State<'b>> {
        self
    }
}
struct FromOldStates<'a, 'c> {
    states: &'a Vec<State<'c>>,
    new_states: &'a mut Vec<State<'c>>,
}
impl<'a, 'b> StateGrouping<'b> for FromOldStates<'a, 'b> {
    fn read(&self) -> &Vec<State<'b>> {
        self.states
    }
    fn write(&mut self) -> &mut Vec<State<'b>> {
        self.new_states
    }
}

fn parse_earley(cfg: &Cfg, src: &[u8], init_sym: u32) -> Ast {
    let mut states = cfg
        .rules_for(init_sym)
        .map(|r| State(0, init_sym, &r.parts[..]))
        .collect::<Vec<_>>();

    // TODO: The completions should sorta have a GC pass. Especially for longer files,
    // most of its content is completely unreferenced.
    // Currently using binary search maps due to the need for range queries, it'd be totally sensible
    // to revisit that
    // let mut completions: Vec<Completion> = vec![];
    // let mut completion_index: Vec<usize> = Vec::with_capacity(src.len());
    // completion_index.push(0);
    let mut completions = Completions::new(src.len());

    for cursor in 0..src.len() {
        println!("@{cursor} {states:?}");
        let mut completions_tx = completions.add_group();
        let mut next_states = vec![];
        let mut new_states = vec![];
        let mut states_before_pass = new_states.len();
        expand_states(
            FromOldStates {
                states: &states,
                new_states: &mut new_states,
            },
            0,
            &mut completions_tx,
            &mut next_states,
            cfg,
            cursor,
            src,
        );

        let mut loop_check = {
            let mut iters = 0;
            move || {
                iters += 1;
                if 120 <= iters {
                    panic!("recursion limit?");
                }
            }
        };
        loop {
            let i = states_before_pass;
            let (sorted, appended) = new_states.split_at_mut(i);
            appended.sort();
            let new_len = dedup_wrt(appended, sorted, |s| s);
            new_states.truncate(i + new_len);
            if states_before_pass == new_states.len() {
                break;
            }
            states_before_pass = new_states.len();
            expand_states(
                &mut new_states,
                i,
                &mut completions_tx,
                &mut next_states,
                cfg,
                cursor,
                src,
            );
            new_states[..states_before_pass].sort();
            loop_check();
        }
        states = next_states;
        states.sort();
        let new_len = dedup(&mut states, |s| s);
        states.truncate(new_len);
    }
    {
        // algo sketch:
        // loop
        //   for every pending state,
        //     follow the backref and create a new state
        //   for every new state, deduplicate
        //   the pending states are the new states
        //   if pending states are none, break
        //
        states.sort();
        let mut pending_start = 0;
        loop {
            let pending_end = states.len();
            for i in pending_start..pending_end {
                let state = states[i];
                if state.remaining.len() == 0 {
                    states.extend(completions.query(state.back_ref, state.sym));
                }
            }
            states[..pending_end].sort();

            let (sorted, appended) = states.split_at_mut(pending_end);
            appended.sort();
            let new_len = dedup_wrt(appended, sorted, |s| s);
            states.truncate(pending_end + new_len);
            if states.len() == pending_end {
                break;
            }
            pending_start = pending_end;
        }
    }
    // the match state is (back_ref: 0, sym: 256), so will always be at the
    // start
    println!("{:?}", states[0]);

    vec![]
}
fn expand_states<'c>(
    mut transfer: impl StateGrouping<'c>,
    i: usize,
    completions: &mut CompletionsTransaction<'c, '_>,
    next_states: &mut Vec<State<'c>>,
    cfg: &'c Cfg,
    cursor: usize,
    src: &[u8],
) {
    for i in i..transfer.read().len() {
        let state = transfer.read()[i];
        let Some(&sym) = state.remaining.get(0) else {
            let new = transfer.write();
            new.extend(completions.query(state.back_ref, state.sym));
            continue;
        };
        if sym < 256 {
            if src[cursor] == sym as u8 {
                next_states.push(State(state.back_ref, state.sym, &state.remaining[1..]));
            }
        } else {
            completions.push((sym, State(state.back_ref, state.sym, &state.remaining[1..])));
            transfer
                .write()
                .extend(cfg.rules_for(sym).map(|r| State(cursor, sym, &r.parts[..])));
        }
    }
}
fn dedup<T, K: PartialEq>(slice: &mut [T], key: impl Fn(&T) -> &K) -> usize {
    let mut write_target = 1;
    for read_head in 1..slice.len() {
        if key(&slice[write_target - 1]) != key(&slice[read_head]) {
            slice.swap(read_head, write_target);
            write_target += 1;
        }
    }
    write_target
}
fn dedup_wrt<T, K: PartialEq + Ord>(slice: &mut [T], wrt: &[T], key: impl Fn(&T) -> &K) -> usize {
    let mut write_target = 0;
    let mut check = 0;
    for read_head in 0..slice.len() {
        let new_val = key(&slice[read_head]);
        if write_target == 0 || key(&slice[write_target - 1]) != new_val {
            while check < wrt.len() && key(&wrt[check]) < new_val {
                check += 1;
            }
            if check == wrt.len() || key(&wrt[check]) != new_val {
                slice.swap(read_head, write_target);
                write_target += 1;
            }
        }
    }
    write_target
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
    let ast = parse_earley(&mycfg, "true or false and b  and not true".as_bytes(), 256);
}
