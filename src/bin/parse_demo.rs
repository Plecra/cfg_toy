fn main() {
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
    cfg_toy::parse_earley(&mycfg, "true or false and not false or bb".as_bytes(), 256, ());
    cfg_toy::parse_earley(&mycfg, "true then".as_bytes(), 256, ());
    let src = "true then".as_bytes();
    let init_sym = 256;
    let mut trace = vec![];
    cfg_toy::parse_earley(&mycfg, src, init_sym, &mut trace);
    for &(start, end, state) in &trace {
        println!("{} {:?}", state_names[state as usize - 256], start..end);
    }
    trace.sort_by_key(|m| (m.1, m.2));
    let ast = cfg_toy::trace_to_ast(&mycfg, src, &trace, init_sym);
    println!("{ast:?}");
}
