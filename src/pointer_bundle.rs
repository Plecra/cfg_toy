// Here's some unfortunate complexity, this boilerplate is just responsible
// for allowing us to read + write to the same reference.
pub(crate) trait BufferPair<T> {
    fn read(&self) -> &[T];
    fn write(&mut self) -> &mut Vec<T>;
}
impl<T, I: BufferPair<T> + ?Sized> BufferPair<T> for &'_ mut I {
    fn read(&self) -> &[T] {
        (**self).read()
    }
    fn write(&mut self) -> &mut Vec<T> {
        (**self).write()
    }
}
impl<T> BufferPair<T> for Vec<T> {
    fn read(&self) -> &[T] {
        self
    }
    fn write(&mut self) -> &mut Vec<T> {
        self
    }
}
pub(crate) struct Transfer<'a, T> {
    pub states: &'a Vec<T>,
    pub new_states: Vec<T>,
}
impl<'a, T> BufferPair<T> for Transfer<'a, T> {
    fn read(&self) -> &[T] {
        self.states
    }
    fn write(&mut self) -> &mut Vec<T> {
        &mut self.new_states
    }
}
