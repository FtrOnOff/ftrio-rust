//! A/B determinism, the pinned cross-language vectors, lifted verbatim from
//! `ftrio-python/tests/unit/test_ab_determinism.py`. The same `user:key[:salt]` must produce the
//! same bucket the .NET and Python runtimes produce.

use ftrio::compute_bucket;

#[test]
fn pinned_cross_language_vectors() {
    // (user_id, toggle_key, salt) -> expected bucket, byte-for-byte identical across all three
    // runtimes.
    assert_eq!(compute_bucket("alice", "TestingABTest", ""), 84);
    assert_eq!(compute_bucket("bob", "TestingABTest", ""), 68);
    assert_eq!(compute_bucket("charlie", "TestingABTest", ""), 89);
    assert_eq!(compute_bucket("dave", "TestingABTest", ""), 64);
    assert_eq!(compute_bucket("alice", "TestingABTestSalted", "round2"), 2);
    assert_eq!(compute_bucket("bob", "TestingABTestSalted", "round2"), 99);
}

#[test]
fn bucket_is_stable_across_repeated_calls() {
    let first = compute_bucket("alice", "TestingABTest", "");
    let second = compute_bucket("alice", "TestingABTest", "");
    assert_eq!(first, second);
}

#[test]
fn salt_changes_the_bucket() {
    // Adding a salt meaningfully alters the assignment (the salted vectors differ from the unsalted).
    let unsalted = compute_bucket("alice", "TestingABTest", "");
    let salted = compute_bucket("alice", "TestingABTest", "round2");
    assert_ne!(unsalted, salted);
}

#[test]
fn bucket_is_always_in_range() {
    // unsigned_abs() % 100 keeps the bucket in 0..=99 even for the i32::MIN edge input (documented in
    // ab_test.rs); this smoke-checks a spread of inputs stays in range.
    for user in ["a", "bb", "ccc", "user-1234", "ZZZ"] {
        let bucket = compute_bucket(user, "SomeToggle", "salt");
        assert!(bucket < 100, "bucket {bucket} out of range for {user}");
    }
}
