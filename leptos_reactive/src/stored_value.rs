use crate::{with_runtime, Runtime, ScopeProperty};
use std::{
    cell::RefCell,
    fmt,
    hash::{Hash, Hasher},
    marker::Unsize,
    ops::CoerceUnsized,
    ptr::NonNull,
    rc::Rc,
};

slotmap::new_key_type! {
    /// Unique ID assigned to a [`StoredValue`].
    pub(crate) struct StoredValueId;
}

/// A **non-reactive** wrapper for any value, which can be created with [`store_value`].
///
/// If you want a reactive wrapper, use [`create_signal`](crate::create_signal).
///
/// This allows you to create a stable reference for any value by storing it within
/// the reactive system. Like the signal types (e.g., [`ReadSignal`](crate::ReadSignal)
/// and [`RwSignal`](crate::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
pub struct StoredValue<T: ?Sized>
where
    T: 'static,
{
    id: StoredValueId,
    inner: NonNull<RefCell<T>>,
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<StoredValue<U>>
    for StoredValue<T>
{
}

impl<T: Default> Default for StoredValue<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: ?Sized> Clone for StoredValue<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for StoredValue<T> {}

impl<T: ?Sized> fmt::Debug for StoredValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredValue")
            .field("id", &self.id)
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T: ?Sized> Eq for StoredValue<T> {}

impl<T: ?Sized> PartialEq for StoredValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: ?Sized> Hash for StoredValue<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Runtime::current().hash(state);
        self.id.hash(state);
    }
}

impl<T: ?Sized> StoredValue<T> {
    /// Returns a clone of the current stored value.
    ///
    /// # Panics
    /// Panics if you try to access a value owned by a reactive node that has been disposed.
    ///
    /// # Examples
    /// ```
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    ///
    /// #[derive(Clone)]
    /// pub struct MyCloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_value(MyCloneableData { value: "a".into() });
    ///
    /// // calling .get_value() clones and returns the value
    /// assert_eq!(data.get_value().value, "a");
    /// // can be `data().value` on nightly
    /// // assert_eq!(data().value, "a");
    /// # runtime.dispose();
    /// ```
    #[track_caller]
    pub fn get_value(&self) -> T
    where
        T: Clone,
    {
        self.try_get_value().expect("could not get stored value")
    }

    /// Same as [`StoredValue::get_value`] but will not panic by default.
    #[track_caller]
    pub fn try_get_value(&self) -> Option<T>
    where
        T: Clone,
    {
        self.try_with_value(T::clone)
    }

    /// Returns a clone of the current stored value.
    ///
    /// # Panics
    /// Panics if you try to access a value owned by a reactive node that has been disposed.
    ///
    /// # Examples
    /// ```
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    ///
    /// #[derive(Clone)]
    /// pub struct MyCloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_value(MyCloneableData { value: "a".into() });
    ///
    /// // calling .get_value() clones and returns the value
    /// assert_eq!(data.get_value().value, "a");
    /// // can be `data().value` on nightly
    /// // assert_eq!(data().value, "a");
    /// # runtime.dispose();
    /// ```
    #[track_caller]
    pub fn get_owned(&self) -> <T as ToOwned>::Owned
    where
        T: ToOwned,
    {
        self.try_get_owned().expect("could not get stored value")
    }

    /// Same as [`StoredValue::get_value`] but will not panic by default.
    #[track_caller]
    pub fn try_get_owned(&self) -> Option<<T as ToOwned>::Owned>
    where
        T: ToOwned,
    {
        self.try_with_value(T::to_owned)
    }

    /// Applies a function to the current stored value and returns the result.
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
    //               track the stored value. This method will also be removed in \
    //               a future version of `leptos`"]
    pub fn with_value<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.try_with_value(f).expect("could not get stored value")
    }

    /// Same as [`StoredValue::with_value`] but returns [`Some(O)]` only if
    /// the stored value has not yet been disposed. [`None`] otherwise.
    pub fn try_with_value<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        let value_ref = self.get_inner()?;
        let value = value_ref.borrow();
        Some(f(&value))
    }

    /// Updates the stored value.
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
    /// data.update_value(|data| data.value = "b".into());
    /// assert_eq!(data.with_value(|data| data.value.clone()), "b");
    /// # runtime.dispose();
    /// ```
    ///
    /// ```
    /// use leptos_reactive::*;
    /// # let runtime = create_runtime();
    ///
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    ///
    /// let data = store_value(MyUncloneableData { value: "a".into() });
    /// let updated = data.try_update_value(|data| {
    ///     data.value = "b".into();
    ///     data.value.clone()
    /// });
    ///
    /// assert_eq!(data.with_value(|data| data.value.clone()), "b");
    /// assert_eq!(updated, Some(String::from("b")));
    /// # runtime.dispose();
    /// ```
    ///
    /// ## Panics
    /// Panics if there is no current reactive runtime, or if the
    /// stored value has been disposed.
    #[track_caller]
    pub fn update_value(&self, f: impl FnOnce(&mut T)) {
        self.try_update_value(f)
            .expect("could not set stored value");
    }

    /// Same as [`Self::update_value`], but returns [`Some(O)`] if the
    /// stored value has not yet been disposed, [`None`] otherwise.
    pub fn try_update_value<O>(self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
        let value_ref = self.get_inner()?;
        let mut value = value_ref.borrow_mut();
        Some(f(&mut value))
    }

    /// Disposes of the stored value
    pub fn dispose(self) {
        _ = with_runtime(|runtime| {
            runtime.stored_values.borrow_mut().remove(self.id);
        });
    }

    /// Sets the stored value.
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
    /// data.set_value(MyUncloneableData { value: "b".into() });
    /// assert_eq!(data.with_value(|data| data.value.clone()), "b");
    /// # runtime.dispose();
    /// ```
    #[track_caller]
    pub fn set_value(&self, value: T)
    where
        T: Sized,
    {
        self.try_set_value(value);
    }

    /// Same as [`Self::set_value`], but returns [`None`] if the
    /// stored value has not yet been disposed, [`Some(T)`] otherwise.
    pub fn try_set_value(&self, value: T) -> Option<T>
    where
        T: Sized,
    {
        with_runtime(|runtime| {
            let n = {
                let values = runtime.stored_values.borrow();
                values.get(self.id).map(Rc::clone)
            };

            if let Some(n) = n {
                let mut n = n.borrow_mut();
                let n = n.downcast_mut::<T>();
                if let Some(n) = n {
                    *n = value;
                    None
                } else {
                    Some(value)
                }
            } else {
                Some(value)
            }
        })
        .ok()
        .flatten()
    }

    /// Cast a [`StoredValue<T>`] to a [`StoredValue<U>`] if the inner value is of type `U`.
    pub fn downcast<U>(self) -> Option<StoredValue<U>> {
        with_runtime(|runtime| {
            let value = {
                let values = runtime.stored_values.borrow();
                values.get(self.id)?.clone()
            };

            if !value.borrow().is::<U>() {
                return None;
            }

            // UNSAFE
            // The cast don't need a surrounding `unsafe` block, but such operation could make `StoredValue<i32> be turned to a `StoredValue<String>` if not carefull
            // SAFETY:
            // The type of the inner value is check above, the ptr is valid as it comes from the runtime stored values map.
            let inner = NonNull::from(&*value).cast::<RefCell<U>>();
            Some(StoredValue { id: self.id, inner })
        })
        .ok()
        .flatten()
    }

    fn get_inner(&self) -> Option<&RefCell<T>> {
        with_runtime(|runtime| {
            let value = {
                let values = runtime.stored_values.borrow();
                NonNull::from(&**values.get(self.id)?)
            };

            let value_ptr = value.as_ptr() as *const ();
            let inner_ptr = self.inner.as_ptr() as *const ();
            // The lazy evaluation is to avoid creating the ref if the backed data don't exist, as it is UB to create invalid ref.
            // SAFETY:
            // The NodeId point to an existing data
            // And that data as the same ptr
            // So the inner ptr must be valid
            #[allow(clippy::unnecessary_lazy_evaluations)]
            (std::ptr::eq(value_ptr, inner_ptr))
                .then(|| unsafe { self.inner.as_ref() })
        })
        .ok()
        .flatten()
    }
}

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
/// let data = store_value(MyUncloneableData { value: "a".into() });
/// let callback_a = move || data.with_value(|data| data.value == "a");
/// let callback_b = move || data.with_value(|data| data.value == "b");
/// # runtime.dispose();
/// ```
///
/// ## Panics
/// Panics if there is no current reactive runtime.
#[track_caller]
pub fn store_value<T>(value: T) -> StoredValue<T>
where
    T: 'static,
{
    let (id, inner) = with_runtime(|runtime| {
        let wrapped_value = Rc::new(RefCell::new(value));
        let inner = NonNull::from(&*wrapped_value);
        let id = runtime.stored_values.borrow_mut().insert(wrapped_value);
        runtime.push_scope_property(ScopeProperty::StoredValue(id));
        (id, inner)
    })
    .expect("store_value failed to find the current runtime");
    StoredValue { id, inner }
}

impl<T> StoredValue<T> {
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
        store_value(value)
    }
}

impl_get_fn_traits!(StoredValue(get_value));

fn test() {
    let v: StoredValue<[i32]> = store_value([1, 2, 3]);
    let _owned_v = v.get_owned();
    v.with_value(|arr| {
        println!("{:?}", arr);
    });

    let f: StoredValue<dyn Fn() -> i32> = store_value(|| 3);
    let _: i32 = f.with_value(|func| func());
}
