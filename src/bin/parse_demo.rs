#![recursion_limit = "2560"]

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
    cfg_toy::parse_earley(
        &mycfg,
        "true or false and not false or bb".as_bytes(),
        256,
        (),
    );
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


    let (json_cfg, state_names) = cfg_toy::cfg! {
        json ws 
        value
        object members member
        array  elements element
        string characters character
        hex escape
        number;
        
        ws ::= .
        ws ::= " " ws.
        ws ::= "\n" ws.

        json ::= element.
        
        value ::= object.
        value ::= array.
        value ::= string.
        value ::= number.
        value ::= "true".
        value ::= "false".
        value ::= "null".
        
        object ::= "{" ws "}".
        object ::= "{" members "}".

        members ::= member.
        members ::= member "," members.
        member ::= ws string ws ":" element.

        array ::= "[" ws "]".
        array ::= "[" elements "]".

        elements ::= element.
        elements ::= element "," elements.
        
        element ::= ws value ws.

        string ::= "\"" characters "\"".
        characters ::= .
        characters ::= character characters.

        character ::= "\\" escape.
        escape ::= "\"" .
        escape ::= "\\" .
        escape ::= "/" .
        escape ::= "b" .
        escape ::= "f" .
        escape ::= "n" .
        escape ::= "r" .
        escape ::= "t" .
        escape ::= "u" hex hex hex hex .
        hex ::= "0" .
        hex ::= "1" .
        hex ::= "2" .
        hex ::= "3" .
        hex ::= "4" .

        hex ::= "5" .
        hex ::= "6" .
        hex ::= "7" .
        hex ::= "8" .
        hex ::= "9" .
        hex ::= "a" .
        hex ::= "b" .
        hex ::= "c" .
        hex ::= "d" .
        hex ::= "e" .
        hex ::= "f" .
        hex ::= "A" .
        hex ::= "B" .
        hex ::= "C" .
        hex ::= "D" .
        hex ::= "E" .
        hex ::= "F" .


    };
    let json_cfg = json_cfg.map(|&sym| {
        cfg_toy::LabelledSymbol {
            symbol: sym,
            // TODO: can make the terminals actually visible if we like
            label: sym.checked_sub(256).map(|idx| state_names[idx as usize]).unwrap_or("terminal"),
        }
    });

    let src = "{}".as_bytes();
    for rule in &json_cfg.rules {
        println!("{rule:?}");
    }
    let init_sym = 256;
    let mut trace = vec![];
    let src = cfg_toy::cast_buf(src);
    cfg_toy::parse_earley(&json_cfg, src, init_sym, &mut trace);
    trace.sort_by_key(|m| (m.1, m.2));
    let ast = cfg_toy::trace_to_ast(&json_cfg, src, &trace, init_sym);
    println!("{ast:?}");
}
