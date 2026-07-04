//! Runnable demonstration of the FtrIO `#[toggle]` attribute.
//!
//! Like the .NET `PlaygroundConsole` and the Python `playground`, this runs an **infinite loop** with
//! a 2-second delay, printing one block per iteration: the current (cycling) user context, then every
//! `#[toggle]`-gated function with its raw config value, resolved decision, and the context used —
//! and it lets you watch the decorated body run or be skipped. `ReloadOnChange` is on, so you can
//! **edit `appsettings.json` live** and watch the decisions change on the next tick. Ctrl+C to exit.
//!
//! Run it (this is a standalone crate, so run it from its own directory):
//! ```text
//! cd ftrio-playground && cargo run
//! cd ftrio-playground && cargo run -- --no-config   # offline-safe default: everything on (one-shot)
//! ```

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ftrio::{
    compute_bucket, toggle, toggle_async, toggle_parser_provider, FtrIoContextAccessor,
    StrategyToggleParser, ToggleParserBuilder,
};

/// A demo user profile the loop cycles through, so context-aware toggles visibly change per user.
struct Profile {
    user: &'static str,
    plan: &'static str,
    country: &'static str,
}

const PROFILES: &[Profile] = &[
    Profile {
        user: "alice",
        plan: "premium",
        country: "NL",
    },
    Profile {
        user: "bob",
        plan: "basic",
        country: "US",
    },
    Profile {
        // carol has a per-user override for `welcome_banner` (see appsettings.json), so this profile
        // visibly demonstrates overrides winning before any strategy.
        user: "carol",
        plan: "premium",
        country: "GB",
    },
];

/// A context accessor whose current user is chosen by a shared index the loop advances each tick.
struct CyclingContext {
    index: Arc<AtomicUsize>,
}

impl CyclingContext {
    fn profile(&self) -> &'static Profile {
        &PROFILES[self.index.load(Ordering::Relaxed) % PROFILES.len()]
    }
}

impl FtrIoContextAccessor for CyclingContext {
    fn get_user_id(&self) -> Option<String> {
        Some(self.profile().user.to_string())
    }
    fn get_attribute(&self, attribute_name: &str) -> Option<String> {
        let profile = self.profile();
        match attribute_name {
            "plan" => Some(profile.plan.to_string()),
            "country" => Some(profile.country.to_string()),
            _ => None,
        }
    }
}

// --- gated functions: the attribute is what a viewer sees gating execution ---

#[toggle]
fn plain_on() {
    println!("      [body] plain_on ran");
}

#[toggle]
fn plain_off() {
    println!("      [body] plain_off ran");
}

#[toggle]
fn half_rollout() {
    println!("      [body] half_rollout ran");
}

#[toggle]
fn blue_slot() {
    println!("      [body] blue_slot ran");
}

#[toggle]
fn green_slot() {
    println!("      [body] green_slot ran");
}

#[toggle]
fn beta_users() {
    println!("      [body] beta_users ran");
}

#[toggle]
fn premium_plan() {
    println!("      [body] premium_plan ran");
}

#[toggle]
fn ab_experiment() {
    println!("      [body] ab_experiment ran");
}

#[toggle_async]
async fn welcome_banner() -> &'static str {
    "welcome banner shown"
}

fn config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("appsettings.json")
}

fn main() {
    if std::env::args().any(|arg| arg == "--no-config") {
        run_offline_demo();
        return;
    }

    let path = config_path();
    let index = Arc::new(AtomicUsize::new(0));
    let context = Arc::new(CyclingContext {
        index: index.clone(),
    });

    let parser = ToggleParserBuilder::new()
        .with_base_path(path.clone())
        .with_percentage_rollout()
        .with_blue_green(
            Some("green".to_string()),
            vec!["blue".to_string(), "green".to_string()],
        )
        .with_context_strategies()
        .with_overrides()
        .with_context_accessor(context)
        .build()
        .expect("build playground parser");
    toggle_parser_provider::configure(Arc::new(parser));

    println!("FtrIO Rust playground");
    println!("config: {}", path.display());
    println!(
        "Looping every 2s, cycling users. Edit appsettings.json live (ReloadOnChange is on). \
         Ctrl+C to exit.\n"
    );

    let mut iteration: usize = 0;
    loop {
        // Select this tick's user, then evaluate everything against it.
        index.store(iteration % PROFILES.len(), Ordering::Relaxed);
        let profile = &PROFILES[iteration % PROFILES.len()];
        let raw_values = load_raw_toggle_values(&path);

        println!(
            "================ tick {iteration} | {} UTC | user={} plan={} country={} \
             ================",
            clock_utc(),
            profile.user,
            profile.plan,
            profile.country
        );
        print_header();

        show("plain_on", "boolean", &raw_values, "-", plain_on);
        show("plain_off", "boolean", &raw_values, "-", plain_off);
        show(
            "half_rollout",
            "percentage",
            &raw_values,
            "probabilistic",
            half_rollout,
        );
        show(
            "blue_slot",
            "blue-green",
            &raw_values,
            "slot=green",
            blue_slot,
        );
        show(
            "green_slot",
            "blue-green",
            &raw_values,
            "slot=green",
            green_slot,
        );
        show(
            "beta_users",
            "user list",
            &raw_values,
            &format!("user={}", profile.user),
            beta_users,
        );
        show(
            "premium_plan",
            "attribute",
            &raw_values,
            &format!("plan={}", profile.plan),
            premium_plan,
        );

        let bucket = compute_bucket(profile.user, "ab_experiment", "");
        show(
            "ab_experiment",
            "a/b test",
            &raw_values,
            &format!("user={} bucket={bucket} < 50 ?", profile.user),
            ab_experiment,
        );

        // Async gated function, awaited either way.
        print_row(
            "welcome_banner",
            "boolean (async)",
            raw_values
                .get("welcome_banner")
                .map(String::as_str)
                .unwrap_or("<absent>"),
            &decision_label("welcome_banner"),
            &format!("user={}", profile.user),
        );
        let banner = block_on(welcome_banner());
        println!(
            "      [body] welcome_banner -> {}",
            if banner.is_empty() {
                "<skipped, default>"
            } else {
                banner
            }
        );

        println!();
        iteration += 1;
        thread::sleep(Duration::from_secs(2));
    }
}

/// Print the query row for a toggle, then invoke its `#[toggle]`-gated function so the viewer sees
/// the body run or be skipped.
fn show(
    key: &str,
    grammar: &str,
    raw_values: &RawValues,
    context_note: &str,
    gated: impl FnOnce(),
) {
    let raw = raw_values
        .get(key)
        .map(String::as_str)
        .unwrap_or("<absent>");
    print_row(key, grammar, raw, &decision_label(key), context_note);
    gated();
}

fn print_header() {
    println!(
        "{:<22} {:<16} {:<34} {:<8} context",
        "toggle key", "grammar", "raw value", "decision"
    );
    println!("{}", "-".repeat(96));
}

fn print_row(key: &str, grammar: &str, raw: &str, decision: &str, context_note: &str) {
    println!("{key:<22} {grammar:<16} {raw:<34} {decision:<8} {context_note}");
}

/// Query the ambient parser for a human-readable decision label.
fn decision_label(key: &str) -> String {
    match toggle_parser_provider::instance().get_toggle_status(key) {
        Ok(true) => "ON".to_string(),
        Ok(false) => "OFF".to_string(),
        Err(error) => format!("ERR({error})"),
    }
}

/// Configure the parser against a path with no config file at all and show the offline-safe default:
/// a normally-off toggle now runs. One-shot (not looped).
fn run_offline_demo() {
    let parser = StrategyToggleParser::from_app_settings("no_such_appsettings_file.json");
    toggle_parser_provider::configure(Arc::new(parser));

    println!("FtrIO Rust playground (offline: no appsettings.json present)");
    println!("Every toggle defaults to ON. `plain_off` — normally false — now runs:\n");
    plain_off();
    println!(
        "\n(decision for plain_off: {})",
        decision_label("plain_off")
    );
}

/// A convenient alias of the raw-values map.
type RawValues = std::collections::HashMap<String, String>;

/// Load the `Toggles` section as raw display strings, straight from the file (independent of the
/// parser) so the viewer sees exactly what is on disk — re-read each tick so live edits show up.
fn load_raw_toggle_values(path: &Path) -> RawValues {
    let mut map = RawValues::new();
    let Ok(contents) = std::fs::read_to_string(path) else {
        return map;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return map;
    };
    if let Some(toggles) = value.get("Toggles").and_then(|t| t.as_object()) {
        for (key, raw) in toggles {
            let display = match raw {
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            map.insert(key.clone(), display);
        }
    }
    map
}

/// The current UTC time-of-day as `HH:MM:SS`, without a date-library dependency.
fn clock_utc() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let seconds_of_day = seconds % 86_400;
    format!(
        "{:02}:{:02}:{:02}",
        seconds_of_day / 3_600,
        (seconds_of_day % 3_600) / 60,
        seconds_of_day % 60
    )
}

/// Minimal executor: the demo futures never truly pend, so a no-op waker with a poll loop drives
/// `#[toggle_async]` results to completion without a runtime dependency.
fn block_on<F: Future>(future: F) -> F::Output {
    fn noop_waker() -> Waker {
        const VTABLE: RawWakerVTable = RawWakerVTable::new(
            |_| RawWaker::new(std::ptr::null(), &VTABLE),
            |_| {},
            |_| {},
            |_| {},
        );
        unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
    }
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::hint::spin_loop(),
        }
    }
}
