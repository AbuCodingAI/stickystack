//! Reminders: any note line that starts with a date, like
//! `10/5/31 I have to go to this event` (month/day/year, optional `3:30pm`),
//! pops up as a desktop notification once that moment arrives.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Reminder {
    pub due: i64,
    pub line: String,
}

const DEFAULT_HOUR: i32 = 9;

fn parse_time(word: &str) -> Option<(i32, i32)> {
    let w = word.to_lowercase();
    let (digits, pm) = match (w.strip_suffix("pm"), w.strip_suffix("am")) {
        (Some(d), _) => (d, Some(true)),
        (_, Some(d)) => (d, Some(false)),
        _ => (w.as_str(), None),
    };
    let (h, m) = digits.split_once(':')?;
    let (mut h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
    match pm {
        Some(true) if h < 12 => h += 12,
        Some(false) if h == 12 => h = 0,
        _ => {}
    }
    ((0..24).contains(&h) && (0..60).contains(&m)).then_some((h, m))
}

pub fn parse(line: &str) -> Option<Reminder> {
    let line = line.trim();
    let (date, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let mut parts = date.split('/');
    let (m, d, y): (i32, i32, i32) = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    if parts.next().is_some() {
        return None;
    }
    let y = if y < 100 { y + 2000 } else { y };
    let (h, min) = rest
        .split_whitespace()
        .next()
        .and_then(parse_time)
        .unwrap_or((DEFAULT_HOUR, 0));
    let due = glib::DateTime::from_local(y, m, d, h, min, 0.0).ok()?.to_unix();
    Some(Reminder { due, line: line.to_string() })
}

fn now() -> i64 {
    glib::DateTime::now_local().map(|t| t.to_unix()).unwrap_or(0)
}

pub fn reminders(body: &str) -> Vec<Reminder> {
    body.lines().filter_map(parse).collect()
}

pub fn any_due(body: &str) -> bool {
    let t = now();
    reminders(body).iter().any(|r| r.due <= t)
}

fn log_path() -> PathBuf {
    glib::user_config_dir().join("stickystack").join("reminded")
}

/// Fires notifications for every reminder that has come due and hasn't been
/// shown yet. Returns the note files that fired, so their windows can refresh.
pub fn check(notes_dir: &Path) -> Vec<String> {
    let mut seen: HashSet<String> = fs::read_to_string(log_path())
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default();
    let t = now();
    let mut fired = Vec::new();
    let mut changed = false;

    for entry in fs::read_dir(notes_dir).into_iter().flatten().flatten() {
        let Ok(name) = entry.file_name().into_string() else { continue };
        let Ok(body) = fs::read_to_string(entry.path()) else { continue };
        for r in reminders(&body).into_iter().filter(|r| r.due <= t) {
            if seen.insert(format!("{name}|{}", r.line)) {
                notify(name.trim_end_matches(".txt"), &r.line);
                changed = true;
                if !fired.contains(&name) {
                    fired.push(name.clone());
                }
            }
        }
    }

    if changed {
        if let Some(dir) = log_path().parent() {
            let _ = fs::create_dir_all(dir);
        }
        let mut out: Vec<_> = seen.into_iter().collect();
        out.sort();
        let _ = fs::write(log_path(), out.join("\n") + "\n");
    }
    fired
}

fn notify(note: &str, line: &str) {
    let _ = Command::new("notify-send")
        .args(["-a", "StickyStack", "-i", "appointment-soon", "-u", "critical"])
        .arg(format!("Reminder · {note}"))
        .arg(line)
        .spawn();
}
