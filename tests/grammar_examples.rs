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
        label: l.checked_sub(256).map(|idx| state_names[idx as usize]).unwrap_or("terminal")
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
    print_ast(&ast, 0);
    panic!();
}
fn print_ast<'a, 'c, S: cfg_toy::CfgSymbol>(ast: &'a [cfg_toy::Node<'c, S>], indent: usize) -> &'a [cfg_toy::Node<'c, S>] {
    let node = &ast[0];
    for _ in 0..indent  {
        print!(" ");
    }
    println!("- {:?} {}..{}", node.transition, node.start, node.end);
    let mut rem = &ast[1..];
    for _ in 0..node.children {
        rem = print_ast(rem, indent + 1);
    }
    rem
}