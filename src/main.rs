#[derive(Debug)]
struct Rule {
    for_nt: u32,
    parts: Vec<u32>,
}
#[derive(Debug)]
struct Cfg {
    rules: Vec<Rule>,
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
    remaining: &'a [u32]
}
fn State(back_ref: usize, sym: NtSymbol, remaining: &[u32]) -> State<'_> {
    State { back_ref, sym, remaining }
}
fn parse_earley(cfg: &Cfg, src: &[u8], init_sym: u32) -> Ast {
    let init_rules = cfg.rules.partition_point(|r| r.for_nt < init_sym);
    
    let mut states = cfg
        .rules[init_rules..].iter()
        .take_while(|r| r.for_nt == init_sym)
        .map(|r| State(0, init_sym, &r.parts[..]))
        .collect::<Vec<_>>();

    // TODO: The completions should sorta have a GC pass. Especially for longer files,
    // most of its content is completely unreferenced.
    let mut completions: Vec<Completion> = vec![];
    let mut completion_ends: Vec<usize> = Vec::with_capacity(src.len());
    
    for cursor in 0..src.len() {
        println!("@{cursor} {states:?}");
        let base = completions.len();
        let mut next_states = vec![];
        let mut new_states = vec![];
        let mut states_before_pass = new_states.len();
        expand_states(&mut states, 0, &mut new_states, |_, r| r, base, &mut completions, &mut completion_ends, &mut next_states, cfg, cursor, src);
        'done: loop {
        for _ in 0..120 {
            let i = states_before_pass;
            let (sorted, appended) = new_states.split_at_mut(i);
            appended.sort();
            let new_len = dedup_wrt(appended, sorted, |s| s);
            new_states.truncate(i + new_len);
            if states_before_pass == new_states.len() {
                break 'done;
            }
            states_before_pass = new_states.len();
            expand_states(&mut new_states, i, &mut Vec::new(), |r, _| r, base, &mut completions, &mut completion_ends, &mut next_states, cfg, cursor, src);
            new_states[..states_before_pass].sort();
        }
        panic!("recursion limit?");
        }
        states = next_states;
        states.sort();
        let new_len = dedup(&mut states, |s| s);
        states.truncate(new_len);
        completions[base..].sort();
        completion_ends.push(completions.len());
        completion_ends.resize(completions.len(), 0);
    }
    // println!("{:?}", states);
    {
        // algo sketch:
        // loop
        //   for every pending state,
        //     follow the backref and create a new state
        //   for every new state, deduplicate
        //   the pending states are the new states
        //   if pending states are none, break
        // 
        let mut pending_start = 0;
        states.sort();
        loop {
            let pending_end = states.len();
            let mut i = pending_start;
            while i < pending_end {
                let state = states[i];
                if state.remaining.len() != 0 {
                    i += 1;
                    continue;
                }
                let end  = completion_ends[state.back_ref];
                let start_of_comps = state.back_ref + completions[state.back_ref..end]
                    .partition_point(|c| c.0 < state.sym);
                states.extend(completions[start_of_comps..]
                    .iter()
                    .take_while(|c| c.0 == state.sym)
                    .map(|c| c.1));
                // println!("{state:?}");
                i += 1;
            }
            states[..pending_end].sort();
            
            let (sorted, appended) = states.split_at_mut(pending_end);
            appended.sort();
            let new_len = dedup_wrt(appended, sorted, |s| s);
            states.truncate(i + new_len);
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
    states: &mut Vec<State<'c>>,
    mut i: usize,
    new_states: &mut Vec<State<'c>>,
    ref_new_states: impl for<'a, 'b> Fn(&'a mut Vec<State<'b>>, &'a mut Vec<State<'b>>) -> &'a mut Vec<State<'b>>,
    base: usize,
    completions: &mut Vec<Completion<'c>>,
    completion_ends: &mut Vec<usize>,
    next_states: &mut Vec<State<'c>>,
    cfg: &'c Cfg,
    cursor: usize,
    src: &[u8],
) {
    let len = states.len();
    while i < len {
        let state = states[i];
        let Some(&sym) = state.remaining.get(0) else {
            let end  = completion_ends[state.back_ref];
            let start_of_comps = state.back_ref + completions[
                state.back_ref..end
                ]
                .partition_point(|c| c.0 < state.sym);
            let new = ref_new_states(states, new_states);
            new.extend(completions[start_of_comps..]
                .iter()
                .take_while(|c| c.0 == state.sym)
                .map(|c| c.1));
            i += 1;
            continue;
        };
        if sym < 256 {
            if src[cursor] == sym as u8 {
                next_states.push(State(state.back_ref, state.sym, &state.remaining[1..]));
            } else {
                // todo!("do nothing? this state is dead?")
            }
        } else {
            let rules = cfg.rules.partition_point(|r| r.for_nt < sym);
            
            completions.push((sym, State(state.back_ref, state.sym, &state.remaining[1..])));
            ref_new_states(states, new_states).extend(cfg
                .rules[rules..].iter()
                .take_while(|r| r.for_nt == sym)
                .map(|r| State(base, sym, &r.parts[..])));
        }
        i += 1;
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