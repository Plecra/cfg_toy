//! Implements a bundle of methods for maintaining ordered sets inside
//! slices and vectors. This is used in the earley parser for deduplication
//! of the parsing states.

/// Create a new sorted set in a vector.
pub fn sorted_set<T: PartialEq + Ord>(vec: &mut Vec<T>) {
    vec.sort();
    retain_with_context(vec, |cx, v| cx.last() != Some(v));
}

/// A view into the vec that allows the owner to read from a range and write to it
/// implementing StateGrouping for use with expand_states
pub(crate) struct InternalSlice<'a, T> {
    pub(crate) slice: &'a mut Vec<T>,
    pub(crate) range: std::ops::Range<usize>,
}
impl<'a, T> super::BufferPair<T> for InternalSlice<'a, T> {
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
    // Just putting an arbitrary bound on the number of iterations we'll
    // try to saturate the reachable set in. Could be way higher
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
    // as long as there are pending states to process,
    while pending_start < states.len() {
        loop_check();
        let pending_end = states.len();
        //   for every pending state,
        //     generate all the states reachable from it in one step
        rel(InternalSlice {
            slice: states,
            range: pending_start..pending_end,
        });
        states[..pending_end].sort();
        //   for every new state, deduplicate
        isolate_new_elements(states, pending_end);
        //   the new states are now pending being processed
        pending_start = pending_end;
    }
}
/// This implements the ability to extend a sorted set with new elements,
/// you should insert a batch of new elements at the end, and give the index
/// where the new elements start. They'll be deduplicated against the old elements.
///
/// However, this function doesn't yet sort the new elements *into* the rest of the set,
/// meaning the `set` vector is left with two sets in 0..old_len and old_len..end.
/// This allows the caller to process the added elements.
pub fn isolate_new_elements<T: Ord>(set: &mut Vec<T>, old_len: usize) {
    let (old, new) = set.split_at_mut(old_len);
    new.sort();
    let mut check = 0;
    let new_len = slice_retain_with_context(new, |cx, new_val| {
        while check < old.len() && old[check] < *new_val {
            check += 1;
        }
        cx.last() != Some(new_val) && (check == old.len() || old[check] != *new_val)
    });
    set.truncate(old_len + new_len);
}

/// This is `Vec::retain`, but the predicate gets a mutable slice of the
/// already retained elements, so it can do more complex checks.
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
