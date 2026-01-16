#[test]
fn simple_logic() {
    let (mut mycfg, state_names) = cfg_toy::cfg! {
        expr and_expr primary alpha ident ws gap and or not ambiguous_1 ambiguous_2;

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

        // Ambiguous rules to test parse forest handling
        ambiguous_1 ::= "then".
        ambiguous_2 ::= "theatre".

        expr ::= and_expr or expr.
        expr ::= and_expr gap ambiguous_1.
        expr ::= and_expr gap ambiguous_2.
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
    cfg_toy::parse_earley(
        &mycfg,
        "true or false and not false or bb".as_bytes(),
        256,
        (),
    );
    cfg_toy::parse_earley(&mycfg, "true then".as_bytes(), 256, ());
    let src = "true then".as_bytes();
    let mut trace = vec![];
    let mycfg = mycfg.map(|&l| cfg_toy::LabelledSymbol {
        symbol: l,
        label: l
            .checked_sub(256)
            .map(|idx| state_names[idx as usize])
            .unwrap_or("terminal"),
    });
    let init_sym = cfg_toy::LabelledSymbol {
        symbol: 256,
        label: state_names[0],
    };
    let src = cfg_toy::cast_buf(src);
    let completions = cfg_toy::parse_earley(&mycfg, src, init_sym.symbol, &mut trace);
    for &(start, end, state) in &trace {
        println!("{} {:?}", state_names[state as usize - 256], start..end);
    }
    trace.sort_by_key(|m| (m.1, m.2));
    let ast = cfg_toy::trace_to_ast(&mycfg, src, &trace, &completions, &init_sym);
    // println!("{ast:?}");
    cfg_toy::print_ast(&ast, 0);
    panic!();
}

/// S ::= A
// ## so there's this possible concern where we take the wrong
// ## path because of aliasing between rules.
// ## we know that we're looking at somewhere that *some* C is
// ## the valid end of a prefix matching the rule we're after,
// ## but the C we find could be from an unrelated branch
// A ::= P C .
// A ::= P "a" C "b".
//
// C ::= "a" "a" .
// C ::= "a" .
//
// P ::= "a". // (just adding this prefix to obscure the start of the rule)
//
//   C
//   |
//  --
// aaab
//
// so here while attempting A#2, choosing this incorrect C would be
// a trap and incorrectly fail the rule.
//
// repetition within a rule never risks this, because we can only be
// looking at the last NT child of the prefix.
//
// so if necessary a fix *is* to remember the branch we're parsing.
// C(aaa) would know that it's returning to A#1 and therefore we dont
// use it.
//
// ooh the completion stores truly exactly the info we need here: C#1
// returns to a completion of `[]` and C#2` returns to a completion of
// `["b"]`. that's exactly the question we're asking "is this a prefix
// to the suffix I have parsed"
#[test]
fn aliased_rules() {
    let grammar = cfg_toy::cfg! {
        a c;
        a ::= c .
        a ::= "a" c "b".

        c ::= "a" "a" .
        c ::= "a" .
    }
    .0;
    let src = "aab".as_bytes();
    let mut trace = vec![];
    let completions = cfg_toy::parse_earley(&grammar, src, 256, &mut trace);
    // for &(start, end, state) in &trace {
    //     println!("{} {:?}", state_names[state as usize - 256], start..end);
    // }
    trace.sort_by_key(|m| (m.1, m.2, -(m.0 as isize)));
    let ast = cfg_toy::trace_to_ast(&grammar, src, &trace, &completions, &256);
    cfg_toy::print_ast(&ast, 0);
    panic!();
}
#[test]
fn right_recursion() {
    let cfg = cfg_toy::cfg! {
        a;
        
        a ::= "a" a .
        a ::= .
    }.0;
    let src = br#"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"#;
    cfg_toy::parse_earley(&cfg, src, 256, ());
    panic!();
}