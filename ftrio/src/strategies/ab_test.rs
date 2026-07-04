//! A/B testing: `ab:50`, `ab:50:round2`.
//!
//! The bucket function is a **byte-exact, cross-language contract**: the same `user:key[:salt]` must
//! produce the same bucket in the .NET, Python, and Rust runtimes. See `compute_bucket` and the
//! pinned determinism vectors in the test suite.

use rand::Rng;
use sha2::{Digest, Sha256};

use super::{ToggleDecisionStrategy, ToggleEvaluationContext};
use crate::error::ToggleError;

const AB_PREFIX: &str = "ab:";

/// Deterministically bucket a user into `0..=99` for a toggle.
///
/// The algorithm is fixed across all three runtimes:
/// 1. Build `"{user_id}:{toggle_key}"`, or `"{user_id}:{toggle_key}:{salt}"` when a salt is given.
/// 2. SHA-256 the UTF-8 bytes.
/// 3. Read the first four digest bytes as a **little-endian signed** `i32`.
/// 4. Take `unsigned_abs() % 100`.
///
/// Step 4 uses [`i32::unsigned_abs`] deliberately: for the ~1-in-4-billion input whose first four
/// bytes equal `i32::MIN`, `i32::abs` would overflow and panic. `unsigned_abs` yields `2147483648`,
/// matching Python's arbitrary-precision `abs`. (.NET throws `OverflowException` on that single
/// input; the six pinned vectors never hit it, and all three runtimes agree on every vector.)
pub fn compute_bucket(user_id: &str, toggle_key: &str, salt: &str) -> u32 {
    let input = if salt.is_empty() {
        format!("{user_id}:{toggle_key}")
    } else {
        format!("{user_id}:{toggle_key}:{salt}")
    };
    let digest = Sha256::digest(input.as_bytes());
    let signed = i32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]);
    signed.unsigned_abs() % 100
}

/// Resolves `ab:<percentage>[:<salt>]`. With a user id present the decision is the stable bucket
/// (`bucket < percentage`); with no user id it falls back to a probabilistic per-call roll, matching
/// the .NET/Python context-free path.
#[derive(Debug, Default, Clone, Copy)]
pub struct AbTestStrategy;

/// Parse the percentage and optional salt from an `ab:` value.
fn parse_ab_value(raw_value: &str) -> Option<(u32, String)> {
    let body = raw_value.trim().strip_prefix(AB_PREFIX)?;
    let mut parts = body.splitn(2, ':');
    let percentage = parts.next()?.trim().parse::<u32>().ok()?;
    let salt = parts.next().unwrap_or("").to_string();
    Some((percentage, salt))
}

impl ToggleDecisionStrategy for AbTestStrategy {
    fn can_handle(&self, raw_value: &str) -> bool {
        parse_ab_value(raw_value).is_some()
    }

    fn should_execute(
        &self,
        raw_value: &str,
        context: &ToggleEvaluationContext,
    ) -> Result<bool, ToggleError> {
        let (percentage, salt) =
            parse_ab_value(raw_value).ok_or_else(|| ToggleError::ParsedOutOfRange {
                raw_value: raw_value.to_string(),
            })?;

        match context.current_user_id() {
            Some(user_id) => {
                let bucket = compute_bucket(&user_id, context.toggle_key, &salt);
                Ok(bucket < percentage)
            }
            None => {
                let roll = rand::thread_rng().gen_range(0..100);
                Ok(roll < percentage)
            }
        }
    }
}
