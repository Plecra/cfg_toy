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
    let ast = cfg_toy::trace_to_ast(&mycfg, src, &trace, &init_sym);
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
            label: sym.checked_sub(256).map(|idx| state_names[idx as usize]).unwrap_or("terminal"),
        }
    });

    let mut src = 
// r#"[{"name": "Adeel Solangi","language": "Sindhi","id": "V59OF92YF627HFY0","bio": "Donec lobortis eleifend condimentum. Cras dictum dolor lacinia lectus vehicula rutrum. Maecenas quis nisi nunc. Nam tristique feugiat est vitae mollis. Maecenas quis nisi nunc.","version": 6.1},{"name": "Afzal Ghaffar","language": "Sindhi","id": "ENTOCR13RSCLZ6KU","bio": "Aliquam sollicitudin ante ligula, eget malesuada nibh efficitur et. Pellentesque massa sem, scelerisque sit amet odio id, cursus tempor urna. Etiam congue dignissim volutpat. Vestibulum pharetra libero et velit gravida euismod.","version": 1.88},{"name": "Aamir Solangi","language": "Sindhi","id": "IAKPO3R4761JDRVG","bio": "Vestibulum pharetra libero et velit gravida euismod. Quisque mauris ligula, efficitur porttitor sodales ac, lacinia non ex. Fusce eu ultrices elit, vel posuere neque.","version": 7.27},{"name": "Abla Dilmurat","language": "Uyghur","id": "5ZVOEPMJUI4MB4EN","bio": "Donec lobortis eleifend condimentum. Morbi ac tellus erat.","version": 2.53},{"name": "Adil Eli","language": "Uyghur","id": "6VTI8X6LL0MMPJCC","bio": "Vivamus id faucibus velit, id posuere leo. Morbi vitae nisi lacinia, laoreet lorem nec, egestas orci. Suspendisse potenti.","version": 6.49},{"name": "Adile Qadir","language": "Uyghur","id": "F2KEU5L7EHYSYFTT","bio": "Duis commodo orci ut dolor iaculis facilisis. Morbi ultricies consequat ligula posuere eleifend. Aenean finibus in tortor vel aliquet. Fusce eu ultrices elit, vel posuere neque.","version": 1.9},{"name": "Abdukerim Ibrahim","language": "Uyghur","id": "LO6DVTZLRK68528I","bio": "Vivamus id faucibus velit, id posuere leo. Nunc aliquet sodales nunc a pulvinar. Nunc aliquet sodales nunc a pulvinar. Ut viverra quis eros eu tincidunt.","version": 5.9},{"name": "Adil Abro","language": "Sindhi","id": "LJRIULRNJFCNZJAJ","bio": "Etiam malesuada blandit erat, nec ultricies leo maximus sed. Fusce congue aliquam elit ut luctus. Etiam malesuada blandit erat, nec ultricies leo maximus sed. Cras dictum dolor lacinia lectus vehicula rutrum. Integer vehicula, arcu sit amet egestas efficitur, orci justo interdum massa, eget ullamcorper risus ligula tristique libero.","version": 9.32},{"name": "Afonso Vilarchan","language": "Galician","id": "JMCL0CXNXHPL1GBC","bio": "Fusce eu ultrices elit, vel posuere neque. Morbi ac tellus erat. Nunc tincidunt laoreet laoreet.","version": 5.21},{"name": "Mark Schembri","language": "Maltese","id": "KU4T500C830697CW","bio": "Nam laoreet, nunc non suscipit interdum, justo turpis vestibulum massa, non vulputate ex urna at purus. Morbi ultricies consequat ligula posuere eleifend. Vivamus id faucibus velit, id posuere leo. Sed laoreet posuere sapien, ut feugiat nibh gravida at. Ut maximus, libero nec facilisis fringilla, ex sem sollicitudin leo, non congue tortor ligula in eros.","version": 3.17},{"name": "Antia Sixirei","language": "Galician","id": "XOF91ZR7MHV1TXRS","bio": "Pellentesque massa sem, scelerisque sit amet odio id, cursus tempor urna. Phasellus massa ligula, hendrerit eget efficitur eget, tincidunt in ligula. Morbi finibus dui sed est fringilla ornare. Duis pellentesque ultrices convallis. Morbi ultricies consequat ligula posuere eleifend.","version": 6.44},{"name": "Aygul Mutellip","language": "Uyghur","id": "FTSNV411G5MKLPDT","bio": "Duis commodo orci ut dolor iaculis facilisis. Nam semper gravida nunc, sit amet elementum ipsum. Donec pellentesque ultrices mi, non consectetur eros luctus non. Pellentesque massa sem, scelerisque sit amet odio id, cursus tempor urna.","version": 9.1},{"name": "Awais Shaikh","language": "Sindhi","id": "OJMWMEEQWMLDU29P","bio": "Nunc aliquet sodales nunc a pulvinar. Ut dictum, ligula eget sagittis maximus, tellus mi varius ex, a accumsan justo tellus vitae leo. Donec pellentesque ultrices mi, non consectetur eros luctus non. Nulla finibus massa at viverra facilisis. Nunc tincidunt laoreet laoreet.","version": 1.59},{"name": "Ambreen Ahmed","language": "Sindhi","id": "5G646V7E6TJW8X2M","bio": "Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae; Etiam consequat enim lorem, at tincidunt velit ultricies et. Ut maximus, libero nec facilisis fringilla, ex sem sollicitudin leo, non congue tortor ligula in eros.","version": 2.35},{"name": "Celtia Anes","language": "Galician","id": "Z53AJY7WUYPLAWC9","bio": "Nullam ac sodales dolor, eu facilisis dui. Maecenas non arcu nulla. Ut viverra quis eros eu tincidunt. Curabitur quis commodo quam.","version": 8.34},{"name": "George Mifsud","language": "Maltese","id": "N1AS6UFULO6WGTLB","bio": "Phasellus tincidunt sollicitudin posuere. Ut accumsan, est vel fringilla varius, purus augue blandit nisl, eu rhoncus ligula purus vel dolor. Donec congue sapien vel euismod interdum. Cras dictum dolor lacinia lectus vehicula rutrum. Phasellus massa ligula, hendrerit eget efficitur eget, tincidunt in ligula.","version": 7.47},{"name": "Ayturk Qasim","language": "Uyghur","id": "70RODUVRD95CLOJL","bio": "Curabitur ultricies id urna nec ultrices. Aliquam scelerisque pretium tellus, sed accumsan est ultrices id. Duis commodo orci ut dolor iaculis facilisis.","version": 1.32},{"name": "Diale Meso","language": "Sesotho sa Leboa","id": "VBLI24FKF7VV6BWE","bio": "Maecenas non arcu nulla. Vivamus id faucibus velit, id posuere leo. Nullam sodales convallis mauris, sit amet lobortis magna auctor sit amet.","version": 6.29}]"#
r#"[{"name": "Adeel Solangi","language": "Sindhi","id": "V59OF92YF627HFY0","bio": "Donec lobortis eleifend condimentum","version": 6.1},{"name": "Afzal Ghaffar","language": "Sindhi","id": "ENTOCR13RSCLZ6KU","bio": "Aliquam sollicitudin.","version": 1.88},{"name": "Aamir Solangi","language": "Sindhi","id": "IAKPO3R4761JDRVG","bio": "Vestibulum pharetra libero  non ex. Fusce eu ultrices elit, vel posuere neque.","version": 7.27}]"#
// r#"[{"name": "Adeel Solangi","language": "Sindhi","id": "V59OF92YF627HFY0","bio": "Donec lobortis eleifend condimentum","version": 6.1}]"#
// r#"[{"bio": "Donec lobortis eleifend condimentum","version": 6.1}]"#
.as_bytes().to_vec();
    for b in &mut src {
        if b.is_ascii_uppercase() {
            *b = b'a' + (*b % 5);
        }
    }
    
    for rule in &json_cfg.rules {
        println!("{rule:?}");
    }
    let init_sym = cfg_toy::LabelledSymbol {
        symbol: 256,
        label: "json",
    };
    let mut trace = vec![];
    let src = cfg_toy::cast_buf(&src);
    // panic!("{:?} {:?}", &src[215..220], &src[220..]);
    cfg_toy::parse_earley(&json_cfg, src, init_sym.symbol, &mut trace);
    trace.sort_by_key(|m| (m.1, m.2));
    let ast = cfg_toy::trace_to_ast(&json_cfg, src, &trace, &init_sym);
    for el in &ast {
        println!("{el:?}");
    }
}
