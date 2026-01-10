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
    let completions = cfg_toy::parse_earley(&mycfg, src, init_sym, &mut trace);
    for &(start, end, state) in &trace {
        println!("{} {:?}", state_names[state as usize - 256], start..end);
    }
    trace.sort_by_key(|m| (m.1, m.2));
    let ast = cfg_toy::trace_to_ast(&mycfg, src, &trace, &completions, &init_sym);
    println!("{ast:?}");

    let (json_cfg, state_names) = cfg_toy::cfg! {
        json
        value
        object members member
        array  elements element
        string characters character
        escape hex
        number integer digits digit onenine fraction exponent sign
        ws af;


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

        character ::= "a" .
        character ::= "b" .
        character ::= "c" .
        character ::= "d" .
        character ::= "e" .
        character ::= "f" .
        character ::= "g" .
        character ::= "h" .
        character ::= "i" .
        character ::= "j" .
        character ::= "k" .
        character ::= "l" .
        character ::= "m" .
        character ::= "n" .
        character ::= "o" .
        character ::= "p" .
        character ::= "q" .
        character ::= "r" .
        character ::= "s" .
        character ::= "t" .
        character ::= "u" .
        character ::= "v" .
        character ::= "w" .
        character ::= "x" .
        character ::= "y" .
        character ::= "z" .
        character ::= "." .
        character ::= "_" .
        character ::= "-" .
        character ::= "/" .
        character ::= ")" .
        character ::= "(" .
        character ::= ";" .
        character ::= ":" .
        character ::= "," .
        character ::= digit .
        character ::= " " .
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
        hex ::= digit .
        hex ::= af .
        hex ::= "A" .
        hex ::= "B" .
        hex ::= "C" .
        hex ::= "D" .
        hex ::= "E" .
        hex ::= "F" .

        number ::= integer fraction exponent .
        integer ::= digit .
        integer ::= onenine digits .
        integer ::= "-" digit .
        integer ::= "-" onenine digits .

        digits ::= .
        digits ::= digit digits.
        digit ::= "0" .
        digit ::= onenine .

        onenine ::= "1" .
        onenine ::= "2" .
        onenine ::= "3" .
        onenine ::= "4" .
        onenine ::= "5" .
        onenine ::= "6" .
        onenine ::= "7" .
        onenine ::= "8" .
        onenine ::= "9" .

        fraction ::= .
        fraction ::= "." digits .

        exponent ::= .
        exponent ::= "e" sign digits .
        exponent ::= "E" sign digits .

        sign ::= .
        sign ::= "+" .
        sign ::= "-" .

        ws ::= .
        ws ::= " " ws.
        ws ::= "\n" ws.


        af ::= "a" .
        af ::= "b" .
        af ::= "c" .
        af ::= "d" .
        af ::= "e" .
        af ::= "f" .
    };
    let json_cfg = json_cfg.map(|&sym| {
        cfg_toy::LabelledSymbol {
            symbol: sym,
            // TODO: can make the terminals actually visible if we like
            label: sym
                .checked_sub(256)
                .map(|idx| state_names[idx as usize])
                .unwrap_or("terminal"),
        }
    });

    let mut src = 
// r#"[{"name": "Adeel Solangi","language": "Sindhi","id": "V59OF92YF627HFY0","bio": "Donec lobortis eleifend condimentum. Cras dictum dolor lacinia lectus vehicula rutrum. Maecenas quis nisi nunc. Nam tristique feugiat est vitae mollis. Maecenas quis nisi nunc.","version": 6.1},{"name": "Afzal Ghaffar","language": "Sindhi","id": "ENTOCR13RSCLZ6KU","bio": "Aliquam sollicitudin ante ligula, eget malesuada nibh efficitur et. Pellentesque massa sem, scelerisque sit amet odio id, cursus tempor urna. Etiam congue dignissim volutpat. Vestibulum pharetra libero et velit gravida euismod.","version": 1.88},{"name": "Aamir Solangi","language": "Sindhi","id": "IAKPO3R4761JDRVG","bio": "Vestibulum pharetra libero et velit gravida euismod. Quisque mauris ligula, efficitur porttitor sodales ac, lacinia non ex. Fusce eu ultrices elit, vel posuere neque.","version": 7.27},{"name": "Abla Dilmurat","language": "Uyghur","id": "5ZVOEPMJUI4MB4EN","bio": "Donec lobortis eleifend condimentum. Morbi ac tellus erat.","version": 2.53},{"name": "Adil Eli","language": "Uyghur","id": "6VTI8X6LL0MMPJCC","bio": "Vivamus id faucibus velit, id posuere leo. Morbi vitae nisi lacinia, laoreet lorem nec, egestas orci. Suspendisse potenti.","version": 6.49},{"name": "Adile Qadir","language": "Uyghur","id": "F2KEU5L7EHYSYFTT","bio": "Duis commodo orci ut dolor iaculis facilisis. Morbi ultricies consequat ligula posuere eleifend. Aenean finibus in tortor vel aliquet. Fusce eu ultrices elit, vel posuere neque.","version": 1.9},{"name": "Abdukerim Ibrahim","language": "Uyghur","id": "LO6DVTZLRK68528I","bio": "Vivamus id faucibus velit, id posuere leo. Nunc aliquet sodales nunc a pulvinar. Nunc aliquet sodales nunc a pulvinar. Ut viverra quis eros eu tincidunt.","version": 5.9},{"name": "Adil Abro","language": "Sindhi","id": "LJRIULRNJFCNZJAJ","bio": "Etiam malesuada blandit erat, nec ultricies leo maximus sed. Fusce congue aliquam elit ut luctus. Etiam malesuada blandit erat, nec ultricies leo maximus sed. Cras dictum dolor lacinia lectus vehicula rutrum. Integer vehicula, arcu sit amet egestas efficitur, orci justo interdum massa, eget ullamcorper risus ligula tristique libero.","version": 9.32},{"name": "Afonso Vilarchan","language": "Galician","id": "JMCL0CXNXHPL1GBC","bio": "Fusce eu ultrices elit, vel posuere neque. Morbi ac tellus erat. Nunc tincidunt laoreet laoreet.","version": 5.21},{"name": "Mark Schembri","language": "Maltese","id": "KU4T500C830697CW","bio": "Nam laoreet, nunc non suscipit interdum, justo turpis vestibulum massa, non vulputate ex urna at purus. Morbi ultricies consequat ligula posuere eleifend. Vivamus id faucibus velit, id posuere leo. Sed laoreet posuere sapien, ut feugiat nibh gravida at. Ut maximus, libero nec facilisis fringilla, ex sem sollicitudin leo, non congue tortor ligula in eros.","version": 3.17},{"name": "Antia Sixirei","language": "Galician","id": "XOF91ZR7MHV1TXRS","bio": "Pellentesque massa sem, scelerisque sit amet odio id, cursus tempor urna. Phasellus massa ligula, hendrerit eget efficitur eget, tincidunt in ligula. Morbi finibus dui sed est fringilla ornare. Duis pellentesque ultrices convallis. Morbi ultricies consequat ligula posuere eleifend.","version": 6.44},{"name": "Aygul Mutellip","language": "Uyghur","id": "FTSNV411G5MKLPDT","bio": "Duis commodo orci ut dolor iaculis facilisis. Nam semper gravida nunc, sit amet elementum ipsum. Donec pellentesque ultrices mi, non consectetur eros luctus non. Pellentesque massa sem, scelerisque sit amet odio id, cursus tempor urna.","version": 9.1},{"name": "Awais Shaikh","language": "Sindhi","id": "OJMWMEEQWMLDU29P","bio": "Nunc aliquet sodales nunc a pulvinar. Ut dictum, ligula eget sagittis maximus, tellus mi varius ex, a accumsan justo tellus vitae leo. Donec pellentesque ultrices mi, non consectetur eros luctus non. Nulla finibus massa at viverra facilisis. Nunc tincidunt laoreet laoreet.","version": 1.59},{"name": "Ambreen Ahmed","language": "Sindhi","id": "5G646V7E6TJW8X2M","bio": "Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae; Etiam consequat enim lorem, at tincidunt velit ultricies et. Ut maximus, libero nec facilisis fringilla, ex sem sollicitudin leo, non congue tortor ligula in eros.","version": 2.35},{"name": "Celtia Anes","language": "Galician","id": "Z53AJY7WUYPLAWC9","bio": "Nullam ac sodales dolor, eu facilisis dui. Maecenas non arcu nulla. Ut viverra quis eros eu tincidunt. Curabitur quis commodo quam.","version": 8.34},{"name": "George Mifsud","language": "Maltese","id": "N1AS6UFULO6WGTLB","bio": "Phasellus tincidunt sollicitudin posuere. Ut accumsan, est vel fringilla varius, purus augue blandit nisl, eu rhoncus ligula purus vel dolor. Donec congue sapien vel euismod interdum. Cras dictum dolor lacinia lectus vehicula rutrum. Phasellus massa ligula, hendrerit eget efficitur eget, tincidunt in ligula.","version": 7.47},{"name": "Ayturk Qasim","language": "Uyghur","id": "70RODUVRD95CLOJL","bio": "Curabitur ultricies id urna nec ultrices. Aliquam scelerisque pretium tellus, sed accumsan est ultrices id. Duis commodo orci ut dolor iaculis facilisis.","version": 1.32},{"name": "Diale Meso","language": "Sesotho sa Leboa","id": "VBLI24FKF7VV6BWE","bio": "Maecenas non arcu nulla. Vivamus id faucibus velit, id posuere leo. Nullam sodales convallis mauris, sit amet lobortis magna auctor sit amet.","version": 6.29}]"#
r#"[{"name": "tala","strapped": "somewhat"}, {"compiler":"rustc","version": 7.27}]"#
// r#"[{"name": "Adeel Solangi","language": "Sindhi","id": "V59OF92YF627HFY0","bio": "Donec lobortis eleifend condimentum","version": 6.1}]"#
// r#"[{"bio": "Donec lobortis eleifend condimentum","version": 6.1}]"#
//r#"[{"a": "b", "c": "d", "e": "f", "g": "h"}]"#
//r#"[1, 2, 3.1e6, 4, 5, 0]"#
.as_bytes().to_vec();
    for b in &mut src {
        if b.is_ascii_uppercase() {
            *b = b'a' + (*b % 5);
        }
    }

    // for rule in &json_cfg.rules {
    //     println!("{rule:?}");
    // }
    let init_sym = cfg_toy::LabelledSymbol {
        symbol: 256,
        label: "json",
    };
    let mut trace = vec![];
    let src = cfg_toy::cast_buf(&src);
    // panic!("{:?} {:?}", &src[215..220], &src[220..]);
    let completions = cfg_toy::parse_earley(&json_cfg, src, init_sym.symbol, &mut trace);
    trace.sort_by_key(|m| (m.1, m.2));
    let ast = cfg_toy::trace_to_ast(&json_cfg, src, &trace, &completions, &init_sym);
    cfg_toy::print_ast(&ast, 0);

    let (bnf_grammar_u32, state_names) = cfg_toy::cfg! {
        grammar rules rule rule_content nonterminal terminal symbols symbol ws gap alpha label characters character escape;
        ws ::= " " .
        ws ::= "\n" .
        gap ::= .
        gap ::= ws gap.

        grammar ::= gap rules gap .
        rules ::= rule .
        rules ::= rule gap rules .
        rule ::= nonterminal gap "::=" rule_content "." .
        rule_content ::= gap symbols gap .
        rule_content ::= gap .
        symbols ::= symbol .
        symbols ::= symbol gap symbols .
        symbol ::= terminal .
        symbol ::= nonterminal .


        nonterminal ::= label nonterminal .
        nonterminal ::= label .
        label ::= "_".
        label ::= alpha.

        terminal ::= "\"" characters "\"" .
        characters ::= .
        characters ::= character characters .
        character ::= alpha .
        character ::= " " .
        character ::= ":" .
        character ::= "=" .
        character ::= "." .
        character ::= escape .
        escape ::= "\\" "\"" .
        escape ::= "\\" "\\" .
        escape ::= "\\" "n" .

        alpha ::= "a" .
        alpha ::= "b" .
        alpha ::= "c" .
        alpha ::= "d" .
        alpha ::= "e" .
        alpha ::= "f" .
        alpha ::= "g" .
        alpha ::= "h" .
        alpha ::= "i" .
        alpha ::= "j" .
        alpha ::= "k" .
        alpha ::= "l" .
        alpha ::= "m" .
        alpha ::= "n" .
        alpha ::= "o" .
        alpha ::= "p" .
        alpha ::= "q" .
        alpha ::= "r" .
        alpha ::= "s" .
        alpha ::= "t" .
        alpha ::= "u" .
        alpha ::= "v" .
        alpha ::= "w" .
        alpha ::= "x" .
        alpha ::= "y" .
        alpha ::= "z" .
    };
    let bnf_grammar = bnf_grammar_u32.map(|&sym| cfg_toy::LabelledSymbol {
        symbol: sym,
        label: sym
            .checked_sub(256)
            .map(|idx| state_names[idx as usize])
            .unwrap_or("terminal"),
    });
    let src_bytes = br#"
        ws ::= " " .
        ws ::= "\n" .
        gap ::= .
        gap ::= ws gap.

        grammar ::= gap rules gap .
        rules ::= rule .
        rules ::= rule gap rules .
        rule ::= nonterminal gap "::=" rule_content "." .
        rule_content ::= gap symbols gap .
        rule_content ::= gap .
        symbols ::= symbol .
        symbols ::= symbol gap symbols .
        symbol ::= terminal .
        symbol ::= nonterminal .


        nonterminal ::= alpha nonterminal .
        nonterminal ::= alpha .
        terminal ::= "\"" characters "\"" .
        characters ::= .
        characters ::= character characters .
        character ::= alpha .
        character ::= " " .
        character ::= ":" .
        character ::= "=" .
        character ::= "." .
        character ::= escape .
        escape ::= "\\" "\"" .
        escape ::= "\\" "\\" .
        escape ::= "\\" "n" .
        
        alpha ::= "a" .
        alpha ::= "b" .
        alpha ::= "c" .
        alpha ::= "d" .
        alpha ::= "e" .
        alpha ::= "f" .
        alpha ::= "g" .
        alpha ::= "h" .
        alpha ::= "i" .
        alpha ::= "j" .
        alpha ::= "k" .
        alpha ::= "l" .
        alpha ::= "m" .
        alpha ::= "n" .
        alpha ::= "o" .
        alpha ::= "p" .
        alpha ::= "q" .
        alpha ::= "r" .
        alpha ::= "s" .
        alpha ::= "t" .
        alpha ::= "u" .
        alpha ::= "v" .
        alpha ::= "w" .
        alpha ::= "x" .
        alpha ::= "y" .
        alpha ::= "z" .
    "#;
    let src = cfg_toy::cast_buf(src_bytes);
    let mut trace = vec![];
    let completions = cfg_toy::parse_earley(&bnf_grammar, src, 256, &mut trace);
    trace.sort_by_key(|m| (m.1, m.2, (m.0 as isize)));
    let ast = cfg_toy::trace_to_ast(
        &bnf_grammar,
        src,
        &trace,
        &completions,
        &cfg_toy::LabelledSymbol {
            symbol: 256,
            label: "grammar",
        },
    );
    cfg_toy::print_ast(&ast, 0);
    fn sample_input_size_growth(src_bytes: &[u8], bnf_grammar_u32: &cfg_toy::grammar::Cfg<u32>) {

        let mut data = vec![];
        let mut bench_content = vec![];
        for n in 32..48 {
            while bench_content.len() < (n * 16 * 1024) {
                bench_content.extend_from_slice(src_bytes);
            }
            let mut trace = vec![];
            println!("testing {n}");
            let start = std::time::Instant::now();
            let completions = cfg_toy::parse_earley(
                &bnf_grammar_u32,
                &bench_content,
                256,
                &mut trace,
            );
            // println!("{trace:?}");
            println!("now tracing {:?} {:?}", trace.len(), bench_content.len());
            trace.sort_by_key(|m| (m.1, m.2, (m.0 as isize)));
            let ast = cfg_toy::trace_to_ast(
                &bnf_grammar_u32,
                &bench_content,
                &trace,
                &completions,
                &256,
            );
            data.push((start.elapsed().as_secs_f64(), n));
        }
        println!("{data:?}");
    }
    sample_input_size_growth(br#"aaaaaaaaaaaaa"#,
        &cfg_toy::cfg! {
            A;
            
            A ::= A "a" .
            A ::= .
        }.0);
    // sample_input_size_growth(br#"
    //     grammar ::= gap rules gap .
    //     rules ::= rule .
    //     rules ::= rule gap rules .
    //     rule ::= nonterminal gap "::=" rule_content "." .
    //     rule_content ::= gap symbols gap .
    //     rule_content ::= gap .
    //     symbols ::= symbol .
    //     symbols ::= symbol gap symbols .
    //     symbol ::= terminal .
    //     symbol ::= nonterminal .
    //     "#, &bnf_grammar_u32);
    // println!("now printing");
    // cfg_toy::print_ast(&ast, 0);

    panic!();
    // for el in &ast {
    //     println!("{el:?}");
    // }
    // This is simplified below to avoid the noise from the gaps
    // let (ambiguous_grammar, state_names) = cfg_toy::cfg! {
    //     expr lt gt generic functioncall primary ident ws gap;

    //     ws ::= " " .
    //     gap ::= .
    //     gap ::= ws gap.

    //     lt ::= gap "<" gap.
    //     gt ::= gap ">" gap.

    //     expr ::= functioncall lt expr .
    //     expr ::= functioncall gt expr .
    //     expr ::= functioncall  .
    //     functioncall ::= functioncall gap "(" gap expr gap ")" .
    //     functioncall ::= functioncall gap "<" gap ident gap ">" .
    //     functioncall ::= primary .
    //     ident ::= "a" .
    //     ident ::= "b" .
    //     ident ::= "c" .
    //     primary ::= "(" gap expr gap ")" .
    //     primary ::= ident .
    // };
    let (ambiguous_grammar, state_names) = cfg_toy::cfg! {
        expr lt gt generic functioncall primary ident;

        lt ::= "<".
        gt ::= ">".

        // Ordered choice works here!
        //   `functioncall` first ==> `f<a>(b)` = `(f<a>)(b)`
        //   comparison first ==> `f<a>(b)` = `f < (a>(b))`
        expr ::= functioncall  .
        expr ::= functioncall lt expr .
        expr ::= functioncall gt expr .
        functioncall ::= functioncall "("  expr ")" .
        functioncall ::= functioncall "<"  ident ">" .
        functioncall ::= primary .
        ident ::= "a" .
        ident ::= "b" .
        ident ::= "c" .
        primary ::= "(" expr ")" .
        primary ::= ident .
    };
    let ambiguous_grammar = ambiguous_grammar.map(|&sym| {
        cfg_toy::LabelledSymbol {
            symbol: sym,
            // TODO: can make the terminals actually visible if we like
            label: sym
                .checked_sub(256)
                .map(|idx| state_names[idx as usize])
                .unwrap_or("terminal"),
        }
    });
    let mut trace = vec![];
    let src = b"a<b>(c)";
    let src = cfg_toy::cast_buf(src);
    let completions = cfg_toy::parse_earley(&ambiguous_grammar, src, 256, &mut trace);
    trace.sort_by_key(|m| (m.1, m.2, m.0));
    trace.dedup(); // empty rules get duplicated in the trace
    for &(start, end, state) in &trace {
        println!("{} {:?}", state_names[state as usize - 256], start..end);
    }
    trace.sort_by_key(|m| (m.1, m.2, (m.0 as isize)));
    let ast = cfg_toy::trace_to_ast(
        &ambiguous_grammar,
        src,
        &trace,
        &completions,
        &cfg_toy::LabelledSymbol {
            symbol: 256,
            // TODO: can make the terminals actually visible if we like
            label: "expr",
        },
    );
    println!("{ast:?}");
    // let (dangling_else, state_names) = cfg_toy::cfg! {
    //     expr lt gt generic functioncall primary ident ws gap;

    //     ws ::= " " .
    //     gap ::= .
    //     gap ::= ws gap.

    //     lt ::= gap "<" gap.
    //     gt ::= gap ">" gap.

    //     expr ::= functioncall lt expr .
    //     expr ::= functioncall gt expr .
    //     expr ::= functioncall  .
    //     functioncall ::= functioncall gap "(" gap expr gap ")" .
    //     functioncall ::= functioncall gap "<" gap ident gap ">" .
    //     functioncall ::= primary .
    //     ident ::= "a" .
    //     ident ::= "b" .
    //     ident ::= "c" .
    //     primary ::= "(" gap expr gap ")" .
    //     primary ::= ident .
    // };

    parse_succeeds(
        &cfg_toy::cfg! {
            start list ;
            start ::= list .
            list ::= "a" .
            list ::= "a" list .
        }
        .0,
        b"aaa",
        256,
    );
    parse_succeeds(
        &cfg_toy::cfg! {
            start flexible ;
            start ::= "a" flexible "c" . // abb succeeds until c.
            start ::= "a" "b" flexible . // abb works with only a single b
            flexible ::= "b" "b" .
            flexible ::= "b"  .
        }
        .0,
        b"abb",
        256,
    );
    //     ```
    // S ::= A B .
    // A ::= "a" "a" . // prefer
    // A ::= "a" .
    // B ::= "a" "b" . // prefer
    // B ::= "b" .
    // ```
    // here is a very simple ambiguour CFG. Taken as a PEG there is only one correct parse.
    // We want an algorithm that is capable of finding the peg parse in general.

    // This highlights the restriction of right-to-left parsing. To form a parse of the `S`
    // rule in the extraction process we must move the cursor through the string, and to
    // achieve linear behaviour we *must* only follow edges that form valid prefixes from
    // the current point. This means that we must choose which `B` we will use before visiting
    // `A`, or learning anything about which `A` parses are valid: that information is not
    // found yet. However, the earley charts have incorrectly recognized `"a"` as a valid parse
    // of `A` in this context, giving us an additional potential B parse. to disambiguate at B
    // we need to discover which B belongs to the parse following the wide `A` prefix. This is,
    // in general, not possible.

    let (grammar, state_names) = cfg_toy::cfg! {
        S A B ;
        S ::= A B .
        A ::= "a" "a" .
        A ::= "a" .
        B ::= "a" "b" .
        B ::= "b" .
    };
    let grammar = grammar.map(|&sym| cfg_toy::LabelledSymbol {
        symbol: sym,
        label: sym
            .checked_sub(256)
            .map(|idx| state_names[idx as usize])
            .unwrap_or("terminal"),
    });
    let mut trace = vec![];
    let src = b"aab";
    let src = cfg_toy::cast_buf(src);
    let completions = cfg_toy::parse_earley(&grammar, src, 256, &mut trace);
    for &(start, end, state) in &trace {
        println!("{} {:?}", state_names[state as usize - 256], start..end);
    }
    trace.sort_by_key(|m| (m.1, m.2, -(m.0 as isize)));
    let ast = cfg_toy::trace_to_ast(
        &grammar,
        src,
        &trace,
        &completions,
        &cfg_toy::LabelledSymbol {
            symbol: 256,
            label: "S",
        },
    );
    println!("{ast:?}");
}

fn parse_succeeds(grammar: &cfg_toy::grammar::Cfg<u32>, src: &[u8], init_sym: u32) {
    let mut trace = vec![];
    let completions = cfg_toy::parse_earley(grammar, src, init_sym, &mut trace);
    trace.sort_by_key(|m| (m.1, m.2, (m.0 as isize)));
    cfg_toy::trace_to_ast(grammar, src, &trace, &completions, &init_sym);
}
