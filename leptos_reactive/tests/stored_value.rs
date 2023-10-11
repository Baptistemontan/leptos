use leptos_reactive::*;
use std::fmt::Display;

#[test]
fn basic_stored_value() {
    let runtime = create_runtime();

    let sv = store_value(0);
    assert_eq!(sv.get_value(), 0);
    sv.set_value(5);
    assert_eq!(sv.get_value(), 5);

    runtime.dispose();
}

#[cfg(feature = "nightly")]
#[test]
fn dyn_stored_value() {
    let runtime = create_runtime();
    let mut i = 0;

    let sv: StoredValue<dyn FnMut() -> i32> = store_value(move || {
        i += 1;
        i
    });

    assert_eq!(sv.update_value(|f| f()), 1);
    assert_eq!(sv.update_value(|f| f()), 2);
    assert_eq!(sv.update_value(|f| f()), 3);

    runtime.dispose();
}

#[cfg(feature = "nightly")]
#[test]
fn unsized_stored_value() {
    let runtime = create_runtime();

    let sv: StoredValue<[i32]> = store_value([1, 2, 3, 4]);

    assert_eq!(sv.to_owned(), [1, 2, 3, 4]);
    sv.update_value(|arr| arr[0] = 45);
    assert_eq!(sv.to_owned(), [45, 2, 3, 4]);

    runtime.dispose();
}

#[cfg(feature = "nightly")]
#[test]
#[should_panic(expected = "could not get stored value")]
fn disposed_unsized_stored_value() {
    let runtime = create_runtime();

    let sv: StoredValue<[i32]> = store_value([1, 2, 3, 4]);

    runtime.dispose();

    let _ = sv.to_owned();
}

#[cfg(feature = "nightly")]
#[test]
fn downcast_unsized_stored_value() {
    let runtime = create_runtime();

    let sv: StoredValue<[i32]> = store_value([1, 2, 3, 4]);

    let casted_sv = sv.downcast::<[i32; 4]>().unwrap();
    casted_sv.set_value([2, 3, 4, 5]);

    assert_eq!(sv.get_owned(), [2, 3, 4, 5]);

    runtime.dispose();
}

#[cfg(feature = "nightly")]
#[test]
#[should_panic(expected = "downcasted to wrong type")]
fn downcast_unsized_stored_value_to_wrong_size() {
    let runtime = create_runtime();

    let sv: StoredValue<[i32]> = store_value([1, 2, 3, 4]);

    let casted_sv =
        sv.downcast::<[i32; 3]>().expect("downcasted to wrong type");
    casted_sv.set_value([2, 3, 4]);

    assert_eq!(sv.get_owned(), [2, 3, 4]);

    runtime.dispose();
}

#[cfg(feature = "nightly")]
#[test]
fn downcast_dyn_stored_value() {
    let runtime = create_runtime();

    let sv: StoredValue<dyn Display> = store_value(String::from("test"));

    let v = sv.with_value(|s| format!("this is a {}", s));

    assert_eq!(v, "this is a test");

    let casted_sv = sv.downcast::<String>().unwrap();

    let v = casted_sv.with_value(|s| format!("this is a {}", s));

    assert_eq!(v, "this is a test");

    runtime.dispose();
}

#[cfg(feature = "nightly")]
#[test]
#[should_panic(expected = "downcasted to wrong type")]
fn downcast_dyn_stored_value_to_wrong_type() {
    let runtime = create_runtime();

    let sv: StoredValue<dyn Debug> = store_value(String::from("test"));

    let casted_sv = sv.downcast::<Vec<u8>>().expect("downcasted to wrong type");

    assert!(casted_sv.with_value(|arr| !arr.is_empty()));

    runtime.dispose();
}
