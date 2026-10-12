//! What a provisional layout change changed, in words — for the
//! keep-or-revert prompt, which otherwise asks "keep these settings?"
//! about settings it never names.
//!
//! Pure: two lists of heads in, one line per output that differs out. The
//! "before" is the revert snapshot, the "after" what the compositor
//! reports now, so the prompt describes what is actually on screen rather
//! than what was asked for.

use crate::types::{Head, Mode, Transform};

/// One line per output that differs between `before` and `after`, in
/// `after`'s order, then any output that has gone. Matched by connector:
/// within one session, which a countdown never outlasts, a connector names
/// one output.
pub fn describe(before: &[Head], after: &[Head]) -> Vec<String> {
    let mut lines = Vec::new();
    for now in after {
        let Some(was) = before.iter().find(|b| b.connector == now.connector) else {
            lines.push(format!("{} connected", now.connector));
            continue;
        };
        if let Some(line) = describe_one(was, now) {
            lines.push(line);
        }
    }
    for was in before {
        if !after.iter().any(|a| a.connector == was.connector) {
            lines.push(format!("{} no longer connected", was.connector));
        }
    }
    lines
}

fn describe_one(was: &Head, now: &Head) -> Option<String> {
    match (was.enabled, now.enabled) {
        (true, false) => return Some(format!("{} turned off", now.connector)),
        (false, true) => return Some(format!("{} turned on at {}", now.connector, mode_words(now.current_mode))),
        (false, false) => return None,
        (true, true) => {}
    }
    let mut parts = Vec::new();
    if !same_mode(was.current_mode, now.current_mode) {
        parts.push(format!("{} → {}", mode_words(was.current_mode), mode_words(now.current_mode)));
    }
    if (was.scale - now.scale).abs() > 1e-3 {
        parts.push(format!("scale {} → {}", number(was.scale), number(now.scale)));
    }
    if was.transform != now.transform {
        parts.push(format!("{} → {}", turn_words(was.transform), turn_words(now.transform)));
    }
    if was.position != now.position {
        parts.push(format!(
            "moved from {}, {} to {}, {}",
            was.position.0, was.position.1, now.position.0, now.position.1
        ));
    }
    (!parts.is_empty()).then(|| format!("{}: {}", now.connector, parts.join("; ")))
}

/// Whether two modes are the same picture: size and refresh, not
/// whether either is the preferred one.
fn same_mode(a: Option<Mode>, b: Option<Mode>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => (a.width, a.height, a.refresh_mhz) == (b.width, b.height, b.refresh_mhz),
        (None, None) => true,
        _ => false,
    }
}

/// `2560×1440 at 144 Hz`; `59.95 Hz` for a rate that isn't whole.
fn mode_words(mode: Option<Mode>) -> String {
    match mode {
        Some(m) => format!("{}×{} at {} Hz", m.width, m.height, number(f64::from(m.refresh_mhz) / 1000.0)),
        None => "no mode".to_string(),
    }
}

/// A number as a person writes it: `1.6`, `2`, `59.95` — at most two
/// places, no trailing zeros.
fn number(n: f64) -> String {
    let s = format!("{n:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn turn_words(t: Transform) -> &'static str {
    match t {
        Transform::Normal => "not rotated",
        Transform::Rotate90 => "rotated 90°",
        Transform::Rotate180 => "rotated 180°",
        Transform::Rotate270 => "rotated 270°",
        Transform::Flipped => "flipped",
        Transform::Flipped90 => "flipped and rotated 90°",
        Transform::Flipped180 => "flipped and rotated 180°",
        Transform::Flipped270 => "flipped and rotated 270°",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Identity;

    fn mode(width: i32, height: i32, refresh_mhz: i32) -> Mode {
        Mode { width, height, refresh_mhz, preferred: false }
    }

    fn head(connector: &str, m: Mode) -> Head {
        Head {
            connector: connector.into(),
            identity: Identity::default(),
            description: String::new(),
            modes: vec![m],
            current_mode: Some(m),
            position: (0, 0),
            transform: Transform::Normal,
            scale: 1.0,
            enabled: true,
        }
    }

    /// The prompt's whole point: each kind of change named, on one line
    /// for its output, in the words of what is on screen now.
    #[test]
    fn every_kind_of_change_is_named_on_its_outputs_line() {
        let was = head("DP-4", mode(1920, 1080, 60_000));
        let now = Head {
            current_mode: Some(mode(2560, 1440, 143_973)),
            scale: 1.6,
            transform: Transform::Rotate90,
            position: (2560, 0),
            ..was.clone()
        };
        assert_eq!(
            describe(&[was], &[now]),
            ["DP-4: 1920×1080 at 60 Hz → 2560×1440 at 143.97 Hz; scale 1 → 1.6; not rotated → rotated 90°; moved from 0, 0 to 2560, 0"]
        );
    }

    /// An output left alone says nothing; one turned off says only that.
    #[test]
    fn only_outputs_that_changed_are_listed() {
        let a = head("eDP-1", mode(2880, 1800, 120_000));
        let b = head("DP-4", mode(2560, 1440, 144_000));
        let off = Head { enabled: false, ..b.clone() };
        assert_eq!(describe(&[a.clone(), b], &[a, off]), ["DP-4 turned off"]);
    }

    /// The preferred flag is not part of what's on screen.
    #[test]
    fn the_same_mode_marked_preferred_is_no_change() {
        let was = head("DP-4", mode(2560, 1440, 144_000));
        let now = Head { current_mode: Some(Mode { preferred: true, ..mode(2560, 1440, 144_000) }), ..was.clone() };
        assert!(describe(&[was], &[now]).is_empty());
    }

    #[test]
    fn an_output_that_went_or_came_is_said_so() {
        let a = head("eDP-1", mode(2880, 1800, 120_000));
        let b = head("DP-4", mode(2560, 1440, 144_000));
        assert_eq!(describe(std::slice::from_ref(&a), std::slice::from_ref(&b)), ["DP-4 connected", "eDP-1 no longer connected"]);
    }
}
