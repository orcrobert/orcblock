use std::fmt::Write as _;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use orcblock_ipc::{Mode, Status};

/// The status as a few lines of text.
pub(crate) fn status(status: &Status, tz: &TimeZone) -> String {
    let mut out = String::new();
    let mode = match status.mode {
        Mode::Live => "",
        Mode::DryRun => " (dry run: nothing is really blocked)",
    };
    let _ = writeln!(out, "OrcBlock helper {}{mode}", status.daemon_version);

    match &status.block {
        Some(block) => {
            let _ = writeln!(
                out,
                "The orc is guarding: {} left, until {}",
                remaining(block.remaining_secs),
                clock(block.ends_at, tz),
            );
        }
        None => {
            let _ = writeln!(out, "The orc is asleep: nothing is blocked");
        }
    }

    if status.sites.is_empty() && status.apps.is_empty() {
        let _ = writeln!(
            out,
            "Nothing listed yet; add a site with `orcblock add youtube.com` or an app with `orcblock add-app <bundle-id>`"
        );
    } else if !status.sites.is_empty() {
        let _ = writeln!(out, "Sites:");
        for site in &status.sites {
            let _ = writeln!(out, "  {}", site.display);
        }
    }

    if !status.apps.is_empty() {
        let _ = writeln!(out, "Apps:");
        for app in &status.apps {
            let _ = writeln!(out, "  {} ({})", app.name, app.bundle_id);
        }
    }

    if !status.blocked_launches.is_empty() {
        let closed: Vec<String> = status
            .blocked_launches
            .iter()
            .map(|launch| format!("{} at {}", launch.name, time(launch.at, tz)))
            .collect();
        let _ = writeln!(out, "Closed by the orc: {}", closed.join(", "));
    }

    let potions = status.potions;
    let _ = writeln!(
        out,
        "Sleep potions: {} of {} left (more on {})",
        potions.available, potions.per_month, potions.refill_on
    );
    out
}

/// `2 h 5 min`, `12 min 30 s` or `45 s`.
pub(crate) fn remaining(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    match (h, m) {
        (0, 0) => format!("{s} s"),
        (0, m) => format!("{m} min {s} s"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

fn clock(at: Timestamp, tz: &TimeZone) -> String {
    at.to_zoned(tz.clone()).strftime("%a %H:%M").to_string()
}

fn time(at: Timestamp, tz: &TimeZone) -> String {
    at.to_zoned(tz.clone()).strftime("%H:%M").to_string()
}

#[cfg(test)]
mod tests;
