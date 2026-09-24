use super::collect_in_name_order;
use std::cell::Cell;
use std::io;
use std::path::PathBuf;

#[test]
fn enumeration_stops_after_the_budget_and_one_overflow_probe() {
    let pulled = Cell::new(0);
    let entries = (0..100_000).map(|index| {
        pulled.set(pulled.get() + 1);
        Ok((format!("entry-{index:06}"), PathBuf::from("unused")))
    });
    let listing = collect_in_name_order(entries, 5);
    assert_eq!(
        pulled.get(),
        6,
        "the remainder must not be enumerated or sorted"
    );
    assert_eq!(listing.named.len(), 5);
    assert!(listing.truncated);
}

#[test]
fn unreadable_entries_consume_the_enumeration_budget() {
    let pulled = Cell::new(0);
    let entries = (0..100_000).map(|_| {
        pulled.set(pulled.get() + 1);
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    });
    let listing = collect_in_name_order(entries, 5);
    assert_eq!(pulled.get(), 6);
    assert!(listing.named.is_empty());
    assert_eq!(listing.unreadable_entries, 5);
    assert!(listing.truncated);
}

#[test]
fn exact_fit_is_complete_and_sorts_the_retained_names() {
    let entries = ["c", "a", "b"]
        .into_iter()
        .map(|name| Ok((name.to_owned(), PathBuf::from(name))));
    let listing = collect_in_name_order(entries, 3);
    assert!(!listing.truncated);
    assert_eq!(
        listing
            .named
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    let empty = collect_in_name_order(std::iter::empty(), 3);
    assert!(!empty.truncated);
    assert!(empty.named.is_empty());
}
