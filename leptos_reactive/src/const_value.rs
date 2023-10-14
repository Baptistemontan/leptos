use crate::{with_runtime, Runtime, ScopeProperty};
use std::{
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    rc::Rc,
};

slotmap::new_key_type! {
    /// Unique ID assigned to a [`ConstValue`].
    pub(crate) struct ConstValueId;
}

/// A **non-reactive** wrapper for const values that don't implement `Clone` or the cloning is expensive, which can be created with [`store_const_value`].
///
/// If you want a reactive wrapper, use [`create_signal`](crate::create_signal).
///
/// If you want mutable access to the value, see [`StoredValue`](crate::StoredValue)
///
/// This allows you to create a stable reference for any value by storing it within
/// the reactive system. Like the signal types (e.g., [`ReadSignal`](crate::ReadSignal)
/// and [`RwSignal`](crate::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
pub struct ConstValue<T>
where
    T: 'static,
{
    id: ConstValueId,
    ty: PhantomData<T>,
}

impl<T: Default> Default for ConstValue<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> Clone for ConstValue<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ConstValue<T> {}

impl<T> fmt::Debug for ConstValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredValue")
            .field("id", &self.id)
            .field("ty", &self.ty)
            .finish()
    }
}

impl<T> Eq for ConstValue<T> {}

impl<T> PartialEq for ConstValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Hash for ConstValue<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Runtime::current().hash(state);
        self.id.hash(state);
    }
}

impl<T> ConstValue<T> {
    /// Returns a Rc to current stored value.
    ///
    /// # Panics
    /// Panics if you try to access a value owned by a reactive node that has been disposed.
    ///
    /// # Examples
    /// ```
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    ///
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_const_value(MyUncloneableData { value: "a".into() });
    ///
    /// // calling .get_value() returns a Rc containing the value
    /// assert_eq!(data.get_value().value, "a");
    /// // can be `data().value` on nightly
    /// // assert_eq!(data().value, "a");
    /// # runtime.dispose();
    /// ```
    #[track_caller]
    pub fn get_value(&self) -> Rc<T> {
        self.try_get_value().expect("could not get const value")
    }

    /// Same as [`ConstValue::get_value`] but will not panic by default.
    #[track_caller]
    pub fn try_get_value(&self) -> Option<Rc<T>> {
        with_runtime(|runtime| {
            let values = runtime.const_values.borrow();
            let value = values.get(self.id).cloned()?;
            value.downcast::<T>().ok()
        })
        .ok()
        .flatten()
    }

    /// Applies a function to the current stored const value and returns the result.
    ///
    /// # Panics
    /// Panics if you try to access a value owned by a reactive node that has been disposed.
    ///
    /// # Examples
    /// ```
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    ///
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_value(MyUncloneableData { value: "a".into() });
    ///
    /// // calling .with_value() to extract the value
    /// assert_eq!(data.with_value(|data| data.value.clone()), "a");
    /// # runtime.dispose();
    /// ```
    #[track_caller]
    //               track the stored const value. This method will also be removed in \
    //               a future version of `leptos`"]
    pub fn with_value<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.try_with_value(f)
            .expect("could not get stored const value")
    }

    /// Same as [`ConstValue::with_value`] but returns [`Some(O)]` only if
    /// the stored value has not yet been disposed. [`None`] otherwise.
    pub fn try_with_value<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        with_runtime(|runtime| {
            let value = {
                let values = runtime.const_values.borrow();
                values.get(self.id)?.clone()
            };
            let value = value.downcast_ref::<T>()?;
            Some(f(value))
        })
        .ok()
        .flatten()
    }

    /// Disposes of the stored const value
    pub fn dispose(self) {
        _ = with_runtime(|runtime| {
            runtime.const_values.borrow_mut().remove(self.id);
        });
    }
}

/// Creates a **non-reactive** wrapper for any value by storing it within
/// the reactive system.
///
/// This wrapper is meant for value that need to be stored once and shared
/// but don't implement `Clone` or the cloning is expensive.
///
/// Like the signal types (e.g., [`ReadSignal`](crate::ReadSignal)
/// and [`RwSignal`](crate::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
/// ```compile_fail
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// // this structure is neither `Copy` nor `Clone`
/// pub struct MyUncloneableData {
///   pub value: String
/// }
///
/// // ❌ this won't compile, as it can't be cloned or copied into the closures
/// let data = MyUncloneableData { value: "a".into() };
/// let callback_a = move || data.value == "a";
/// let callback_b = move || data.value == "b";
/// # runtime.dispose();
/// ```
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// // this structure is neither `Copy` nor `Clone`
/// pub struct MyUncloneableData {
///     pub value: String,
/// }
///
/// // ✅ you can move the `StoredValue` and access it with .with_value()
/// let data = store_const_value(MyUncloneableData { value: "a".into() });
/// let callback_a = move || data.with_value(|data| data.value == "a");
/// let callback_b = move || data.with_value(|data| data.value == "b");
///
/// // ✅ Or ypu can use `.get_value` to get back a `Rc` to your value:
/// let callback_a = move || data.get_value().value == "a";
///
/// # runtime.dispose();
/// ```
///
/// ## Panics
/// Panics if there is no current reactive runtime.
#[track_caller]
pub fn store_const_value<T>(value: T) -> ConstValue<T>
where
    T: 'static,
{
    let id = with_runtime(|runtime| {
        let id = runtime.const_values.borrow_mut().insert(Rc::new(value));
        runtime.push_scope_property(ScopeProperty::ConstValue(id));
        id
    })
    .expect("store_value failed to find the current runtime");
    ConstValue {
        id,
        ty: PhantomData,
    }
}

impl<T> ConstValue<T> {
    /// Creates a **non-reactive** wrapper for any value by storing it within
    /// the reactive system.
    ///
    /// Like the signal types (e.g., [`ReadSignal`](crate::ReadSignal)
    /// and [`RwSignal`](crate::RwSignal)), it is `Copy` and `'static`. Unlike the signal
    /// types, it is not reactive; accessing it does not cause effects to subscribe, and
    /// updating it does not notify anything else.
    /// ```compile_fail
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    /// // this structure is neither `Copy` nor `Clone`
    /// pub struct MyUncloneableData {
    ///   pub value: String
    /// }
    ///
    /// // ❌ this won't compile, as it can't be cloned or copied into the closures
    /// let data = MyUncloneableData { value: "a".into() };
    /// let callback_a = move || data.value == "a";
    /// let callback_b = move || data.value == "b";
    /// # runtime.dispose();
    /// ```
    /// ```
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    /// // this structure is neither `Copy` nor `Clone`
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    ///
    /// // ✅ you can move the `StoredValue` and access it with .with_value()
    /// let data = StoredValue::new(MyUncloneableData { value: "a".into() });
    /// let callback_a = move || data.with_value(|data| data.value == "a");
    /// let callback_b = move || data.with_value(|data| data.value == "b");
    /// # runtime.dispose();
    /// ```
    ///
    /// ## Panics
    /// Panics if there is no current reactive runtime.
    #[inline(always)]
    #[track_caller]
    pub fn new(value: T) -> Self {
        store_const_value(value)
    }
}

impl_get_fn_traits!(ConstValue(get_value) @ Rc<T>);
