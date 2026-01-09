// Here's some unfortunate complexity, this boilerplate is just responsible
// for allowing us to read + write to the same reference.
pub(crate) trait StateGrouping<T> {
    fn read(&self) -> &[T];
    fn write(&mut self) -> &mut Vec<T>;
}
impl<T, I: StateGrouping<T> + ?Sized> StateGrouping<T> for &'_ mut I {
    fn read(&self) -> &[T] {
        (**self).read()
    }
    fn write(&mut self) -> &mut Vec<T> {
        (**self).write()
    }
}
use super::State;
impl<'b> StateGrouping<State<'b>> for Vec<State<'b>> {
    fn read(&self) -> &[State<'b>] {
        self
    }
    fn write(&mut self) -> &mut Vec<State<'b>> {
        self
    }
}
pub(crate) struct FromOldStates<'a, 'c> {
    pub states: &'a Vec<State<'c>>,
    pub new_states: Vec<State<'c>>,
}
impl<'a, 'b> StateGrouping<State<'b>> for FromOldStates<'a, 'b> {
    fn read(&self) -> &[State<'b>] {
        self.states
    }
    fn write(&mut self) -> &mut Vec<State<'b>> {
        &mut self.new_states
    }
}
