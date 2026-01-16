#[derive(Debug)]
pub struct Rule<Symbol> {
    pub for_nt: u32,
    pub parts: Vec<Symbol>,
}
#[derive(Debug)]
pub struct Cfg<Symbol> {
    pub rules: Vec<Rule<Symbol>>,
    pub rule_nullable: Vec<bool>,
    // pub nt_to_nullable_rules_index: Vec<usize>,
    // pub nt_to_nullable_rules_index_offsets: Vec<usize>,
    pub nt_nullable: Vec<bool>,
    pub nt_index: Vec<usize>,
}
impl<Symbol> Cfg<Symbol> {
    /// Needs to preserve nullability
    pub fn map<U>(&self, mut f: impl FnMut(&Symbol) -> U) -> Cfg<U> {
        Cfg {
            rules: self
                .rules
                .iter()
                .map(|rule| Rule {
                    for_nt: rule.for_nt,
                    parts: rule.parts.iter().map(&mut f).collect(),
                })
                .collect(),
            nt_index: self.nt_index.clone(),
            rule_nullable: self.rule_nullable.clone(),
            nt_nullable: self.nt_nullable.clone(),
            // nt_to_nullable_rules_index: self.nt_to_nullable_rules_index.clone(),
            // nt_to_nullable_rules_index_offsets: self.nt_to_nullable_rules_index_offsets.clone(),
        }
    }
}
fn nullable_closure<Symbol: crate::CfgSymbol>(rules: &[Rule<Symbol>]) -> (Vec<bool>, Vec<bool>) {
    let mut rule_nullable = vec![];
    let mut nt_nullable = vec![];
    for rule in rules {
        let is_nullable = rule.parts.is_empty();
        rule_nullable.push(is_nullable);
        let nt = rule.for_nt as usize;
        while nt_nullable.len() <= nt {
            nt_nullable.push(false);
        }
        if is_nullable {
            nt_nullable[nt] = true;
        }
    }
    {
        let mut dirty = true;
        while dirty {
            dirty = false;
            for (i, rule) in rules.iter().enumerate() {
                if rule_nullable[i] {
                    continue;
                }
                if rule.parts.iter().all(|part| match part.as_part() {
                    Ok(_terminal) => false,
                    Err(nt_sym) => nt_nullable[nt_sym as usize],
                }) {
                    rule_nullable[i] = true;
                    nt_nullable[rule.for_nt as usize] = true;
                    dirty = true;
                }
            }
        }
    }
    (rule_nullable, nt_nullable)
}
impl<Symbol: super::CfgSymbol> Cfg<Symbol> {
    pub fn new(mut rules: Vec<Rule<Symbol>>) -> Self {
        rules.sort_by_key(|rule| rule.for_nt);
        let mut nt_index = vec![];
        for (i, rule) in rules.iter().enumerate() {
            let nt = rule.for_nt as usize;
            while nt_index.len() < nt {
                nt_index.push(i);
            }
        }
        nt_index.push(rules.len());

        let (rule_nullable, nt_nullable) = nullable_closure(&rules);

        // let mut nt_to_nullable_rules_index = rules.iter().enumerate().filter(|t| {
        //     rule_nullable[t.0]
        // }).map(|(i, _)| i).collect::<Vec<_>>();
        // nt_to_nullable_rules_index.sort_by_key(|&i| rules[i].for_nt);
        // let mut nt_to_nullable_rules_index_offsets = vec![];
        // for (i, nri) in nt_to_nullable_rules_index.iter().enumerate() {
        //     let nt = rules[*nri].for_nt as usize;
        //     while nt_to_nullable_rules_index_offsets.len() <= nt {
        //         nt_to_nullable_rules_index_offsets.push(i);
        //     }
        // }

        Self {
            rules,
            rule_nullable,
            nt_nullable,
            nt_index,
            // nt_to_nullable_rules_index,
            // nt_to_nullable_rules_index_offsets,
        }
    }
    // pub(crate) fn query_nullable(&self, nt: u32) -> Option<std::ops::Range<usize>> {
    //     let nt = nt as usize;
    //     let end = *self.nt_to_nullable_rules_index_offsets.get(nt)?;
    //     let start = nt.checked_sub(1).and_then(|i| self.nt_to_nullable_rules_index_offsets.get(i)).copied().unwrap_or(0);
    //     Some(start..end)
    // }
    pub(crate) fn query_nt(&self, nt: u32) -> Option<std::ops::Range<usize>> {
        let nt = nt as usize;
        // println!("{nt:?} {:?}", self.nt_index);
        let end = *self.nt_index.get(nt)?;
        let start = nt
            .checked_sub(1)
            .and_then(|i| self.nt_index.get(i))
            .copied()
            .unwrap_or(0);
        Some(start..end)
    }
    pub(crate) fn rules_for(&self, nt: u32) -> impl Iterator<Item = &'_ Rule<Symbol>> + '_ {
        self.rules[self.query_nt(nt).unwrap()].iter()
    }
}
#[macro_export]
macro_rules! cfg_rules {
    {$cx:ident $rule_name:ident $($t:tt)*} => {
        $cx.1.push($rule_name);
        $crate::cfg_rules!($cx $($t)*)
    };
    {$cx:ident $literal:literal $($t:tt)*} => {
        $cx.1.extend($literal.as_bytes().iter().map(|&b| b as u32));
        $crate::cfg_rules!($cx $($t)*);
    };
    {$cx:ident . $rulename:ident :: = $($t:tt)*} => {
        $cx.0.push($crate::grammar::Rule {
            parts: std::mem::take(&mut $cx.1),
            for_nt: $cx.2,
        });
        $cx.2 = $rulename;
        $crate::cfg_rules!($cx $($t)*);
    };
    {$cx:ident .} => {
        $cx.0.push($crate::grammar::Rule {
            parts: std::mem::take(&mut $cx.1),
            for_nt: $cx.2,
        });
    };
}
#[macro_export]
macro_rules! cfg {
    {
        $($states:ident)*;
        $first_rule:ident ::= $($rule_definition:tt)*
    } => {{
        let mut state_names: Vec<&'static str> = vec![];
        let mut states = 256u32;
        $(let $states = states; #[allow(unused_assignments)] { states += 1; }; state_names.push(stringify!($states));)*
        let mut cx: (Vec<$crate::grammar::Rule<u32>>, Vec<u32>, u32) = (vec![], vec![], $first_rule);
        $crate::cfg_rules!(cx $($rule_definition)*);
        ($crate::grammar::Cfg::new(cx.0), state_names)
    }};
}
