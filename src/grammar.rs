
#[derive(Debug)]
pub struct Rule {
    pub for_nt: u32,
    pub parts: Vec<u32>,
}
#[derive(Debug)]
pub struct Cfg {
    pub rules: Vec<Rule>,
}
impl Cfg {
    pub(crate) fn rules_for(&self, nt: u32) -> impl Iterator<Item = &'_ Rule> + '_ {
        let start = self.rules.partition_point(|r| r.for_nt < nt);
        let end = self.rules.partition_point(|r| r.for_nt <= nt);
        // TODO: bench? this is the same as iterating while we're in the group
        // self
        //     .rules[start..].iter()
        //     .take_while(|r| r.for_nt == nt)
        self.rules[start..end].iter()
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
        let mut states = 256u32;
        $(let $states = states; #[allow(unused_assignments)] { states += 1; })*
        let mut cx: (Vec<$crate::grammar::Rule>, Vec<u32>, u32) = (vec![], vec![], $first_rule);
        $crate::cfg_rules!(cx $($rule_definition)*);
        $crate::grammar::Cfg { rules: cx.0 }
    }};
}