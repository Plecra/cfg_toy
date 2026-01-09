
pub struct InternalSlice<'a, T> {
    slice: &'a mut Vec<T>,
    range: std::ops::Range<usize>,
}
impl<'a, T> super::StateGrouping<T> for InternalSlice<'a, T> {
    fn read(&self) -> &[T] {
        &self.slice[self.range.clone()]
    }
    fn write(&mut self) -> &mut Vec<T> {
        self.slice
    }
}
// Find the transitive closure of a relation
pub fn grow_ordered_set<T: Ord + Clone>(
    states: &mut Vec<T>,
    mut rel: impl FnMut(InternalSlice<'_, T>),
) {
    let mut loop_check = {
        let mut iters = 0;
        move || {
            iters += 1;
            if 120 <= iters {
                panic!("recursion limit?");
            }
        }
    };
    let mut pending_start = 0;
    while pending_start < states.len() {
        loop_check();
        let pending_end = states.len();
        rel(InternalSlice {
            slice: states,
            range: pending_start..pending_end,
        });
        states[..pending_end].sort();
        isolate_new_elements(states, pending_end);
        pending_start = pending_end;
    }
}
pub fn isolate_new_elements<T: Ord>(states: &mut Vec<T>, old_len: usize) {
    let (old, new) = states.split_at_mut(old_len);
    new.sort();
    let mut check = 0;
    let new_len = slice_retain_with_context(new, |cx, new_val| {
        while check < old.len() && old[check] < *new_val {
            check += 1;
        }
        cx.last() != Some(new_val) && (check == old.len() || old[check] != *new_val)
    });
    states.truncate(old_len + new_len);
}
fn vec_dedup<T: PartialEq>(vec: &mut Vec<T>) {
    retain_with_context(vec, |cx, v| cx.last() != Some(v));
}
pub fn sorted_set<T: PartialEq + Ord>(vec: &mut Vec<T>) {
    vec.sort();
    vec_dedup(vec);
}
fn slice_retain_with_context<T>(
    vec: &mut [T],
    mut f: impl FnMut(&mut [T], &mut T) -> bool,
) -> usize {
    let mut write = 0;
    for read in 0..vec.len() {
        let (retained, tail) = vec.split_at_mut(write);
        let current = &mut tail[read - write];
        if f(retained, current) {
            vec.swap(write, read);
            write += 1;
        }
    }
    write
}
fn retain_with_context<T>(vec: &mut Vec<T>, f: impl FnMut(&mut [T], &mut T) -> bool) {
    let len = slice_retain_with_context(&mut vec[..], f);
    vec.truncate(len);
}