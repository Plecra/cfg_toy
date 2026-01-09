#[test]
fn simple_logic() {
    let mut mycfg = cfg_toy::cfg! {
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
    let _ = cfg_toy::parse_earley(&mycfg, "true or false and not false or bb".as_bytes(), 256);
    let _ = cfg_toy::parse_earley(&mycfg, "true then".as_bytes(), 256);
}
