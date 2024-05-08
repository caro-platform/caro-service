pub struct This<T: Send + Sync + 'static> {
    pub pointer: *mut T,
}

impl<T: Send + Sync> This<T> {
    pub fn get(&self) -> &mut T {
        unsafe { self.pointer.as_mut().unwrap() }
    }
}

impl<T: Send + Sync> Copy for This<T> {}
impl<T: Send + Sync> Clone for This<T> {
    fn clone(&self) -> Self {
        *self
    }
}

unsafe impl<T: Send + Sync> Send for This<T> {}
unsafe impl<T: Send + Sync> Sync for This<T> {}
