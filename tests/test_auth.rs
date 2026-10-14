use recv::auth::{SessionStore, constant_time_eq};

#[test]
fn constant_time_eq_matches_equal() {
    assert!(constant_time_eq(b"pairing-token", b"pairing-token"));
}

#[test]
fn constant_time_eq_rejects_different() {
    assert!(!constant_time_eq(b"pairing-token", b"pairing-tokeN"));
    assert!(!constant_time_eq(b"pairing-token", b"pairing-toke"));
    assert!(!constant_time_eq(b"", b"x"));
}

#[test]
fn session_store_roundtrip() {
    let store = SessionStore::new();
    let id = store.create();
    assert!(store.is_valid(&id));
    assert!(!store.is_valid("unknown"));
}

#[test]
fn session_store_issues_distinct_ids() {
    let store = SessionStore::new();
    let a = store.create();
    let b = store.create();
    assert_ne!(a, b);
    assert!(store.is_valid(&a));
    assert!(store.is_valid(&b));
}
