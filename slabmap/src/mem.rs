//! Copied from bimap-rs source code as of rev 3dca651620845a939ee9e5393c0a8fe9fe0a1656
//!
//! https://github.com/billyrieger/bimap-rs/blob/3dca651620845a939ee9e5393c0a8fe9fe0a1656/src/mem.rs
//!
//! > bimap-rs is dual-licensed under the [Apache License](https://github.com/billyrieger/bimap-rs/blob/3dca651620845a939ee9e5393c0a8fe9fe0a1656/LICENSE_APACHE) and the [MIT License](https://github.com/billyrieger/bimap-rs/blob/3dca651620845a939ee9e5393c0a8fe9fe0a1656/LICENSE_MIT). As a library user, this means that you are free to choose either license when using bimap-rs. As a library contributor, this means that any work you contribute to bimap-rs will be similarly dual-licensed.
use core::{borrow::Borrow, fmt};
use std::rc::Rc;

#[derive(Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Ref<T>(pub Rc<T>);

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> fmt::Debug for Ref<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Wrapper<T: ?Sized>(pub T);

impl<T: ?Sized> Wrapper<T> {
    pub fn wrap(value: &T) -> &Self {
        // safe because Wrapper<T> is #[repr(transparent)]
        unsafe { &*(value as *const T as *const Self) }
    }
}

impl<K, Q> Borrow<Wrapper<Q>> for Ref<K>
where
    K: Borrow<Q>,
    Q: ?Sized,
{
    fn borrow(&self) -> &Wrapper<Q> {
        // Rc<K>: Borrow<K>
        let k: &K = self.0.borrow();
        // K: Borrow<Q>
        let q: &Q = k.borrow();

        Wrapper::wrap(q)
    }
}
