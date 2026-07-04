//! Small shared utilities: UTC timestamp formatting and atomic file writes.

use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// The current time as a UTC ISO-8601 string (`YYYY-MM-DDTHH:MM:SSZ`), computed without pulling in a
/// date library (Howard Hinnant's civil-from-days algorithm).
pub fn utc_iso8601() -> String {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_seconds = since_epoch.as_secs() as i64;

    let days = total_seconds.div_euclid(86_400);
    let seconds_of_day = total_seconds.rem_euclid(86_400);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_position + 2) / 5 + 1;
    let month = if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    };
    let calendar_year = if month <= 2 { year + 1 } else { year };

    format!("{calendar_year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Write `contents` to `path` atomically via a sibling temp file and a rename.
pub fn write_atomically(path: &Path, contents: &str) -> io::Result<()> {
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp_path = std::path::PathBuf::from(temp);
    std::fs::write(&temp_path, contents)?;
    std::fs::rename(&temp_path, path)?;
    Ok(())
}
