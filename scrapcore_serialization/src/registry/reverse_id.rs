use duplicate::duplicate_item;
use crate::registry::{CollectionItemId, ItemCollection};

/// Trait for looking up the original collection item type by its ID type
pub trait ReverseId {
    type Item;
}

impl<T> ReverseId for CollectionItemId<T> {
    type Item = T;
}

#[duplicate_item(
    ty;
    [Option]; [Vec];
    [Box]; [std::rc::Rc]; [std::cell::RefCell];
    [std::sync::Mutex]; [std::sync::RwLock]; [std::sync::Arc];
)]
impl <R: ReverseId> ReverseId for ty<R> {
    type Item = R::Item;
}

impl <R: ReverseId> ReverseId for &R {
    type Item = R::Item;
}

impl <R: ReverseId, S> ReverseId for std::collections::HashSet<R, S> {
    type Item = R::Item;
}