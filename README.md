## CFG Parser

This is literally just an earley parser impl. Mainly to teach myself at this point,
but also built to be especially tweakable. In particular I'm planning to build the ACFG
parser on it.

Performance is pretty abysmal at the moment, but I'm impressed with the simplicitly of the
algorithm. it's fully functional with really just 3 or 4 loops.

extending it to define splat's grammar is also very much on my mind. With the affix grammar
support it shouldn't be wildly difficult, the task is about matching prefixes with given
parameters.

Add[baseline, w1, new_baseline, new_ws] ::=
    Mul[baseline, w1, new_baseline, new_ws] InlineWs + InlineWs Add[new_baseline, new_ws, _, _]

Sadly I dont think I'm missing a trick haha, the parameters are a bit crazy.
