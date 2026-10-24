//! One function per status segment, each returning the chip(s) to render
//! as `(icon, label)` pairs, or `None` if the segment's optional
//! dependency (paru, a battery, pactl, ...) isn't available - ported
//! 1:1 from the old flexiflow_bar bash script's `seg_*` functions.
//!
//! A few of those functions were missing an icon glyph (plain blank
//! padding where one should've been: kernel, cpu, desktop, a connected
//! network, and both clock icons) - this port fills those in rather
//! than reproducing the gap.

use std::{
    fs,
    path::Path,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

/// What the segments carry from one tick to the next.
#[derive(Default)]
pub struct State {
    cpu: CpuState,
    updates: UpdatesState,
}

#[derive(Default)]
struct CpuState {
    prev_total: u64,
    prev_idle: u64,
}

#[derive(Default)]
struct UpdatesState {
    /// Latest `paru -Qu` count, `None` until the first check finishes.
    count: Arc<Mutex<Option<usize>>>,
    /// Set while a check is running on its background thread.
    checking: Arc<AtomicBool>,
    /// `pacman_db_stamp` as of the start of the latest check.
    checked_stamp: Option<SystemTime>,
    checked_at: Option<Instant>,
}

/// Dispatch by config-file segment name to the matching `seg_*`
/// function below, returning the chip(s) to render with the caller's
/// chosen accent color.
pub fn render(name: &str, state: &mut State) -> Option<Vec<(String, String)>> {
    match name {
        "updates" => updates(&mut state.updates).map(|p| vec![p]),
        "kernel" => kernel().map(|p| vec![p]),
        "battery" => battery().map(|p| vec![p]),
        "ram" => ram().map(|p| vec![p]),
        "cpu" => cpu(&mut state.cpu).map(|p| vec![p]),
        "keyboard" => keyboard().map(|p| vec![p]),
        "desktop" => desktop().map(|p| vec![p]),
        "network" => network().map(|p| vec![p]),
        "volume" => volume().map(|p| vec![p]),
        "clock" => clock(),
        other => {
            log::warn!("flexiflow_bar: unknown segment {other:?} in config");
            None
        }
    }
}

fn command_exists(name: &str) -> bool {
    std::env::var("PATH").is_ok_and(|path_var| {
        path_var.split(':').any(|dir| Path::new(dir).join(name).is_file())
    })
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// How long a count stays trusted while pacman's databases haven't
/// changed. Only AUR releases can show up without touching them, and
/// every check costs an AUR RPC request (rate-limited per IP), so this
/// is the one part that still has to poll.
const AUR_RECHECK: Duration = Duration::from_secs(15 * 60);

const PACMAN_DB: &str = "/var/lib/pacman";

/// Re-runs `paru -Qu` whenever pacman's databases change - so installing
/// updates or syncing (`-Sy`) shows up on the very next tick - and every
/// `AUR_RECHECK` otherwise. The check runs on a background thread, so a
/// slow AUR request never stalls the bar; until the first one finishes
/// the label is "..".
fn updates(state: &mut UpdatesState) -> Option<(String, String)> {
    if !command_exists("paru") {
        return None;
    }
    let stamp = pacman_db_stamp(Path::new(PACMAN_DB));
    if needs_check(state.checked_stamp, stamp, state.checked_at)
        && !state.checking.swap(true, Ordering::AcqRel)
    {
        state.checked_stamp = stamp;
        state.checked_at = Some(Instant::now());
        let count = Arc::clone(&state.count);
        let checking = Arc::clone(&state.checking);
        std::thread::spawn(move || {
            // `paru -Qu` exits non-zero when there's nothing to update
            let n = run("paru", &["-Qu"])
                .map(|s| s.lines().filter(|l| !l.is_empty()).count())
                .unwrap_or(0);
            *count.lock().unwrap() = Some(n);
            checking.store(false, Ordering::Release);
        });
    }
    let label = updates_label(*state.count.lock().unwrap());
    Some(("\u{F019}".to_string(), label))
}

/// A check is due if pacman's databases changed since the last one
/// started (a change mid-check is caught on the first tick after it),
/// or the AUR side has gone `AUR_RECHECK` without one.
fn needs_check(
    checked_stamp: Option<SystemTime>,
    stamp: Option<SystemTime>,
    checked_at: Option<Instant>,
) -> bool {
    checked_stamp != stamp
        || checked_at.is_none_or(|t| t.elapsed() >= AUR_RECHECK)
}

/// Newest mtime across pacman's local and sync databases: the local
/// one changes whenever a package is installed, upgraded or removed,
/// and the sync ones whenever they're refreshed - every way the repo
/// update count can change. A handful of stat()s, cheap every tick.
fn pacman_db_stamp(db: &Path) -> Option<SystemTime> {
    let sync = db.join("sync");
    let mut paths = vec![db.join("local"), sync.clone()];
    if let Ok(entries) = fs::read_dir(&sync) {
        paths.extend(entries.flatten().map(|e| e.path()));
    }
    paths
        .iter()
        .filter_map(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
        .max()
}

fn updates_label(count: Option<usize>) -> String {
    match count {
        None => "..".to_string(),
        Some(0) => "Fully Updated".to_string(),
        Some(1) => "1 Update".to_string(),
        Some(n) => format!("{n} Updates"),
    }
}

fn kernel() -> Option<(String, String)> {
    let release = fs::read_to_string("/proc/sys/kernel/osrelease").ok()?;
    Some(("\u{F17C}".to_string(), release.trim().to_string()))
}

fn battery() -> Option<(String, String)> {
    let dir = ["BAT0", "BAT1", "BAT"]
        .iter()
        .map(|b| Path::new("/sys/class/power_supply").join(b))
        .find(|p| p.is_dir())?;
    let charge: u32 = fs::read_to_string(dir.join("capacity")).ok()?.trim().parse().ok()?;
    let status = fs::read_to_string(dir.join("status")).ok()?;
    let status = status.trim();

    Some((battery_icon(status, charge).to_string(), format!("{charge}%")))
}

fn battery_icon(status: &str, charge: u32) -> &'static str {
    if status != "Discharging" {
        "\u{F0084}"
    } else if charge > 90 {
        "\u{F0079}"
    } else if charge > 80 {
        "\u{F0082}"
    } else if charge > 70 {
        "\u{F0081}"
    } else if charge > 60 {
        "\u{F0080}"
    } else if charge > 50 {
        "\u{F007F}"
    } else if charge > 40 {
        "\u{F007E}"
    } else if charge > 30 {
        "\u{F007D}"
    } else if charge > 20 {
        "\u{F007C}"
    } else if charge > 10 {
        "\u{F007B}"
    } else {
        "\u{F007A}"
    }
}

fn ram() -> Option<(String, String)> {
    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;
    let (mut total, mut available) = (None, None);
    for line in meminfo.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total = v.trim().trim_end_matches("kB").trim().parse::<u64>().ok();
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            available = v.trim().trim_end_matches("kB").trim().parse::<u64>().ok();
        }
    }
    let used_kib = total?.saturating_sub(available?) as f64;
    let label = if used_kib >= 1024.0 * 1024.0 {
        format!("{:.1}G", used_kib / 1024.0 / 1024.0)
    } else {
        format!("{:.0}M", used_kib / 1024.0)
    };
    Some(("\u{F035B}".to_string(), label))
}

/// `user + nice + system + idle` only, same simplified total the old
/// bash version used (ignores iowait/irq/softirq/steal).
fn read_proc_stat() -> Option<(u64, u64)> {
    let s = fs::read_to_string("/proc/stat").ok()?;
    let mut it = s.lines().next()?.split_whitespace();
    it.next()?; // "cpu"
    let user: u64 = it.next()?.parse().ok()?;
    let nice: u64 = it.next()?.parse().ok()?;
    let system: u64 = it.next()?.parse().ok()?;
    let idle: u64 = it.next()?.parse().ok()?;
    Some((user + nice + system + idle, idle))
}

/// Unlike the bash version (which slept 0.5s *inside* this one
/// segment every tick, on top of the main loop's own sleep), this
/// measures the delta since the *previous tick* - one reading per
/// loop iteration, no extra sleep.
fn cpu(state: &mut CpuState) -> Option<(String, String)> {
    let (total, idle) = read_proc_stat()?;
    let (prev_total, prev_idle) = (state.prev_total, state.prev_idle);
    state.prev_total = total;
    state.prev_idle = idle;

    let label = match cpu_pct(prev_total, prev_idle, total, idle) {
        Some(pct) => format!("{pct}%"),
        None => "..".to_string(),
    };
    Some(("\u{F2DB}".to_string(), label))
}

/// `None` on the first tick (no previous reading yet) or if the
/// counters didn't advance.
fn cpu_pct(prev_total: u64, prev_idle: u64, total: u64, idle: u64) -> Option<u64> {
    if prev_total == 0 || total <= prev_total {
        return None;
    }
    let elapsed = total - prev_total;
    let busy = elapsed.saturating_sub(idle.saturating_sub(prev_idle));
    Some(100 * busy / elapsed)
}

fn keyboard() -> Option<(String, String)> {
    let out = run("setxkbmap", &["-query"])?;
    let layout =
        out.lines().find_map(|l| l.strip_prefix("layout:"))?.trim().to_uppercase();
    Some(("\u{F030C}".to_string(), layout))
}

fn desktop() -> Option<(String, String)> {
    let name = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("XDG_SESSION_DESKTOP"))
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "Unknown".to_string());
    Some(("\u{F108}".to_string(), name))
}

fn network() -> Option<(String, String)> {
    for entry in fs::read_dir("/sys/class/net").ok()?.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("wl") {
            continue;
        }
        let Ok(state) = fs::read_to_string(entry.path().join("operstate")) else {
            continue;
        };
        return Some(if state.trim() == "up" {
            ("\u{F05A9}".to_string(), "Connected".to_string())
        } else {
            ("\u{F05AA}".to_string(), "Disconnected".to_string())
        });
    }
    None
}

fn volume() -> Option<(String, String)> {
    if !command_exists("pactl") {
        return None;
    }
    let mute = run("pactl", &["get-sink-mute", "@DEFAULT_SINK@"])?;
    if mute.contains("yes") {
        return Some(("\u{F075F}".to_string(), "muted".to_string()));
    }

    let vol_line = run("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])?;
    let vol: u32 =
        vol_line.split_whitespace().nth(4)?.trim_end_matches('%').parse().ok()?;

    Some((volume_icon(vol).to_string(), format!("{vol}%")))
}

fn volume_icon(vol: u32) -> &'static str {
    if vol >= 75 {
        "\u{F028}"
    } else if vol >= 50 {
        "\u{F057E}"
    } else if vol >= 25 {
        "\u{F0580}"
    } else {
        "\u{F057F}"
    }
}

fn clock() -> Option<Vec<(String, String)>> {
    let day: u32 = run("date", &["+%-d"])?.parse().ok()?;
    let date_label = run("date", &[&format!("+%-d{} %b %Y", day_suffix(day))])?;
    let time_label = run("date", &["+%I:%M %P"])?;
    Some(vec![
        ("\u{F073}".to_string(), date_label),
        ("\u{F017}".to_string(), time_label),
    ])
}

fn day_suffix(day: u32) -> &'static str {
    match day {
        1 | 21 | 31 => "st",
        2 | 22 => "nd",
        3 | 23 => "rd",
        _ => "th",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_icon_picks_charging_regardless_of_level() {
        assert_eq!(battery_icon("Charging", 5), "\u{F0084}");
        assert_eq!(battery_icon("Full", 100), "\u{F0084}");
    }

    #[test]
    fn battery_icon_steps_down_by_ten_while_discharging() {
        assert_eq!(battery_icon("Discharging", 95), "\u{F0079}");
        assert_eq!(battery_icon("Discharging", 55), "\u{F007F}");
        assert_eq!(battery_icon("Discharging", 10), "\u{F007A}");
        assert_eq!(battery_icon("Discharging", 0), "\u{F007A}");
    }

    #[test]
    fn volume_icon_thresholds() {
        assert_eq!(volume_icon(100), "\u{F028}");
        assert_eq!(volume_icon(75), "\u{F028}");
        assert_eq!(volume_icon(74), "\u{F057E}");
        assert_eq!(volume_icon(50), "\u{F057E}");
        assert_eq!(volume_icon(49), "\u{F0580}");
        assert_eq!(volume_icon(25), "\u{F0580}");
        assert_eq!(volume_icon(24), "\u{F057F}");
        assert_eq!(volume_icon(0), "\u{F057F}");
    }

    #[test]
    fn day_suffix_handles_the_11_12_13_exceptions() {
        assert_eq!(day_suffix(1), "st");
        assert_eq!(day_suffix(21), "st");
        assert_eq!(day_suffix(31), "st");
        assert_eq!(day_suffix(2), "nd");
        assert_eq!(day_suffix(3), "rd");
        assert_eq!(day_suffix(11), "th");
        assert_eq!(day_suffix(12), "th");
        assert_eq!(day_suffix(13), "th");
        assert_eq!(day_suffix(4), "th");
    }

    #[test]
    fn cpu_pct_is_none_on_first_tick() {
        assert_eq!(cpu_pct(0, 0, 1000, 800), None);
    }

    #[test]
    fn cpu_pct_computes_busy_fraction_of_the_delta() {
        // 1000 ticks elapsed, 200 of them idle -> 80% busy
        assert_eq!(cpu_pct(5000, 4000, 6000, 4200), Some(80));
        // fully idle delta -> 0% busy
        assert_eq!(cpu_pct(5000, 4000, 6000, 5000), Some(0));
    }

    #[test]
    fn updates_label_is_dots_until_the_first_check_finishes() {
        assert_eq!(updates_label(None), "..");
        assert_eq!(updates_label(Some(0)), "Fully Updated");
        assert_eq!(updates_label(Some(1)), "1 Update");
        assert_eq!(updates_label(Some(3)), "3 Updates");
    }

    #[test]
    fn needs_check_on_first_tick_and_whenever_the_db_changes() {
        let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let t1 = t0 + Duration::from_secs(1);
        let now = Some(Instant::now());

        assert!(needs_check(None, Some(t0), None));
        assert!(!needs_check(Some(t0), Some(t0), now));
        assert!(needs_check(Some(t0), Some(t1), now));
    }

    #[test]
    fn needs_check_once_the_aur_recheck_interval_passes() {
        let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let Some(long_ago) =
            Instant::now().checked_sub(AUR_RECHECK + Duration::from_secs(1))
        else {
            return; // monotonic clock started too recently to go back
        };
        assert!(needs_check(Some(t0), Some(t0), Some(long_ago)));
    }

    #[test]
    fn pacman_db_stamp_tracks_the_newest_local_or_sync_change() {
        let db = std::env::temp_dir()
            .join(format!("flexiflow-bar-db-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&db);
        fs::create_dir_all(db.join("local")).unwrap();
        fs::create_dir_all(db.join("sync")).unwrap();
        let at = |secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        let set = |p: &Path, t| {
            fs::File::open(p).unwrap().set_modified(t).unwrap();
        };

        let core = db.join("sync").join("core.db");
        fs::write(&core, "").unwrap();
        set(&db.join("local"), at(100));
        set(&db.join("sync"), at(100));
        set(&core, at(100));
        assert_eq!(pacman_db_stamp(&db), Some(at(100)));

        // `pacman -Sy` rewriting a sync db
        set(&core, at(200));
        assert_eq!(pacman_db_stamp(&db), Some(at(200)));

        // installing/upgrading/removing a package
        set(&db.join("local"), at(300));
        assert_eq!(pacman_db_stamp(&db), Some(at(300)));

        let _ = fs::remove_dir_all(&db);
    }
}
