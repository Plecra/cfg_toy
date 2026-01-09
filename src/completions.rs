use super::{Completion, NtSymbol, State};
/// Semantically, this is a `BTreeMap<(usize, NtSymbol), State<'a>>`
/// It's implemented via a flat buffer containing all the entries in correct order,
/// and the completion index for locating each value of `usize`. This
/// provides range queries for `(i, sym)` efficiently.
pub(crate) struct Completions<'a> {
    completions: Vec<Completion<'a>>,
    completion_index: Vec<usize>,
}
impl<'a> Completions<'a> {
    pub(crate) fn new(len: usize) -> Self {
        let completions = vec![];
        let mut completion_index = Vec::with_capacity(len + 1);
        completion_index.push(0);
        Self {
            completions,
            completion_index,
        }
    }
    pub(crate) fn add_group(&mut self) -> CompletionsTransaction<'a, '_> {
        CompletionsTransaction::new(self)
    }
    pub(crate) fn query(&self, back_ref: usize, sym: NtSymbol) -> impl Iterator<Item = State<'a>> + '_ {
        let start = self.completion_index[back_ref];
        let end = self.completion_index[back_ref + 1];
        let start_of_comps = start + self.completions[start..end].partition_point(|c| c.0 < sym);
        let end_of_comps = start + self.completions[start..end].partition_point(|c| c.0 <= sym);
        self.completions[start_of_comps..end_of_comps]
            .iter()
            .map(|c| c.1)
    }
}
// Appending a new group to the Completions buffer needs to be careful to
// clean up once it's done, which this type is responsible for.
pub(crate) struct CompletionsTransaction<'a, 'b> {
    completions: &'b mut Completions<'a>,
    start_len: usize,
}
impl<'a, 'b> CompletionsTransaction<'a, 'b> {
    fn new(completions: &'b mut Completions<'a>) -> Self {
        let start_len = completions.completions.len();
        Self {
            completions,
            start_len,
        }
    }
    pub(crate) fn query(&self, back_ref: usize, sym: NtSymbol) -> impl Iterator<Item = State<'a>> + '_ {
        self.completions.query(back_ref, sym)
    }
    pub(crate) fn push(&mut self, completion: Completion<'a>) {
        self.completions.completions.push(completion);
    }
    pub(crate) fn batch_id(&self) -> usize {
        self.completions.completion_index.len() - 1
    }
}
impl<'a, 'b> Drop for CompletionsTransaction<'a, 'b> {
    fn drop(&mut self) {
        self.completions.completions[self.start_len..].sort();
        self.completions
            .completion_index
            .push(self.completions.completions.len());
    }
}
