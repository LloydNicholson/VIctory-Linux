//! The pipelines earlier runs drew with, made while the title boots, with a
//! loading bar on its picture.
//!
//! After a change of the translator (or of the driver) every pipeline the
//! world draws with is compiled again, and the first runs spent their first
//! minutes in the world at 0.1 to 3.5 fps, dropping hundreds of draws a
//! frame, with up to 960 pipelines in the making at once (run xtx0: 135 s
//! of it; 289 pipelines take a second or more, one 47 s). The early list
//! (`library`) starts them when the title creates them, which is while the
//! world loads, after the start screen. Here they are made as soon as the
//! title creates its first pipeline, from the book (`known`), while the
//! title sits on its first screens waiting for A: the long ones first, on
//! the workers and two more (cores less two in all), through the pipeline
//! library, so that the world finds every one in it.
//!
//! A small box on the picture (`crate::host::dots`) says "preparing
//! shaders" with how far it is, over a thin bar. Nothing holds A back: the
//! game starts at once, slowly until the bar is full.
//!
//! What is done is written down by key for the translator, settings and
//! driver it was made with (`known::fingerprint`, in `warm-done.txt`, as
//! `victory warm` does): a start with nothing new shows no box. A compile
//! that does not come back ([`deadline`]) is left to itself and named in
//! `warm-stuck.txt`; one that was being made when the run before ended
//! (`prepare-journal.txt`: a crash in the compiler, or the run was ended) is
//! named there too, and neither is made at the start again for that
//! fingerprint. The title still makes them if it draws with them, as it did
//! before.
//!
//! `VICTORY_PREPARE=0`: nothing is made at the start, and no box.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::make::{self, Pending};

const DONE_FILE: &str = "warm-done.txt";
const BAD_FILE: &str = "warm-bad.txt";
const STUCK_FILE: &str = "warm-stuck.txt";
const EARLY_FILE: &str = "early-pipelines.txt";
const JOURNAL_FILE: &str = "prepare-journal.txt";

/// What a pipeline not known to take long is taken to take, in ms, for the
/// time left.
const PLAIN_MS: f64 = 300.0;
/// How long the box stays full after the last one: long enough to see,
/// short enough not to sit on the title's own screens.
const READY_FOR: Duration = Duration::from_secs(2);

fn on() -> bool {
    std::env::var_os("VICTORY_PREPARE").is_none_or(|v| v != "0")
}

/// How far it is.
#[derive(Clone, Debug)]
pub(crate) struct Progress {
    pub(crate) total: usize,
    pub(crate) done: usize,
    /// Of `done`, those given up on ([`deadline`]).
    pub(crate) stuck: usize,
    /// Expected ms of all, and of those done.
    pub(crate) weight: f64,
    pub(crate) done_weight: f64,
    pub(crate) finished: Option<Instant>,
}

static PROGRESS: Mutex<Option<Progress>> = Mutex::new(None);

fn progress() -> std::sync::MutexGuard<'static, Option<Progress>> {
    PROGRESS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Characters in the box's line: the words on the left, the share on the right.
pub(crate) const COLUMNS: usize = 24;

impl Progress {
    /// How full the bar is: by the expected time of what is done, not by
    /// the count (the long ones go first, and a count crawls through those
    /// and then leaps).
    pub(crate) fn share(&self) -> f64 {
        if self.weight > 0.0 {
            (self.done_weight / self.weight).clamp(0.0, 1.0)
        } else if self.total == 0 {
            1.0
        } else {
            self.done as f64 / self.total as f64
        }
    }

    /// What the box shows: its one line and how full the bar under it is;
    /// full for a moment at the end, then nothing (no box).
    pub(crate) fn shown(&self, now: Instant) -> Option<(String, f64)> {
        let share = match self.finished {
            Some(finished) if now.saturating_duration_since(finished) >= READY_FOR => return None,
            Some(_) => 1.0,
            None => self.share(),
        };
        let percent = format!("{}%", (share * 100.0) as u32);
        Some((format!("{:<w$}{percent:>4}", "PREPARING SHADERS", w = COLUMNS - 4), share))
    }
}

/// What the loading box shows now (none: no box).
pub(crate) fn shown() -> Option<(String, f64)> {
    progress().as_ref().and_then(|p| p.shown(Instant::now()))
}

/// The title created a pipeline: the first time, the pipelines to make are
/// looked for and made, on a thread of their own.
pub(super) fn kick() {
    static KICKED: AtomicBool = AtomicBool::new(false);
    if KICKED.swap(true, Ordering::Relaxed) || !on() {
        return;
    }
    let Some(dir) = super::library::folder() else { return };
    let spawned = std::thread::Builder::new().name("victory-prepare".into()).spawn(move || run(&dir));
    if let Err(e) = spawned {
        tracing::warn!(error = %e, "prepare: no thread, nothing made at the start");
    }
}

/// The keys a file names for `fingerprint` (lines `<fingerprint> <key> ...`),
/// or every line's first word with `fingerprint` `None`.
fn keys_in(path: &Path, fingerprint: Option<&str>) -> HashSet<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            match fingerprint {
                Some(f) => (words.next() == Some(f)).then(|| words.next()).flatten(),
                None => words.next(),
            }
        })
        .map(str::to_owned)
        .collect()
}

fn append(path: &Path, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let written = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| f.write_all(lines.iter().map(|l| format!("{l}\n")).collect::<String>().as_bytes()));
    if let Err(e) = written {
        tracing::warn!(error = %e, file = %path.display(), "prepare: not written down");
    }
}

/// Which of `used` to make: those the book has, not done, bad or stuck for
/// this fingerprint, those known to take long first (longest first).
pub(crate) fn pick(
    used: &[String],
    in_book: &HashSet<String>,
    skip: &HashSet<String>,
    slow_ms: impl Fn(&str) -> Option<u64>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut picked: Vec<String> = used
        .iter()
        .filter(|k| in_book.contains(*k) && !skip.contains(*k) && seen.insert(k.as_str()))
        .cloned()
        .collect();
    // (Stable: the others stay in the early list's order.)
    picked.sort_by_key(|k| std::cmp::Reverse(slow_ms(k).unwrap_or(0)));
    picked
}

/// How long a compile may take before it is given up on: a minute, or three
/// times what it took before.
pub(crate) fn deadline(known_ms: Option<u64>) -> Duration {
    Duration::from_millis(known_ms.map_or(0, |ms| ms * 3)).max(Duration::from_secs(60))
}

fn run(dir: &Path) {
    let began = Instant::now();
    let fingerprint = super::known::fingerprint();
    let path = |name: &str| -> PathBuf { dir.join(name) };
    // Being made when the run before ended: not made at the start again.
    let journal = keys_in(&path(JOURNAL_FILE), None);
    if !journal.is_empty() {
        tracing::warn!(
            pipelines = journal.len(),
            "prepare: pipelines were being made when the run before ended; they are left to the game from now on"
        );
        let lines: Vec<String> =
            journal.iter().map(|k| format!("{fingerprint} {k} being made when the run before ended")).collect();
        append(&path(STUCK_FILE), &lines);
        let _ = std::fs::remove_file(path(JOURNAL_FILE));
    }
    let used: Vec<String> = std::fs::read_to_string(path(EARLY_FILE))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();
    let mut skip = keys_in(&path(DONE_FILE), Some(&fingerprint));
    let done_before = skip.len();
    skip.extend(keys_in(&path(STUCK_FILE), Some(&fingerprint)));
    skip.extend(keys_in(&path(BAD_FILE), None));
    let in_book: HashSet<String> = super::known::keys(dir).into_iter().collect();
    let todo = pick(&used, &in_book, &skip, super::library::slow_ms);
    if todo.is_empty() {
        tracing::info!(
            used = used.len(),
            in_book = in_book.len(),
            done_before,
            fingerprint,
            "prepare: nothing to make at the start"
        );
        return;
    }
    let wanted: HashSet<String> = todo.iter().cloned().collect();
    let mut entries: HashMap<String, super::known::Entry> =
        super::known::entries(dir, &wanted).into_iter().map(|e| (e.0.clone(), e)).collect();
    // (In `todo`'s order: the long ones first.)
    let work: Vec<(String, Arc<Pending>, Option<u64>)> = todo
        .iter()
        .filter_map(|key| {
            let (_, shaders, graphics) = entries.remove(key)?;
            Some((key.clone(), make::shared(Pending::new(shaders, graphics)), super::library::slow_ms(key)))
        })
        .collect();
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let weight_of = |ms: Option<u64>| ms.map_or(PLAIN_MS, |ms| ms as f64);
    tracing::info!(
        pipelines = work.len(),
        long = work.iter().filter(|w| w.2.is_some()).count(),
        used = used.len(),
        in_book = in_book.len(),
        done_before,
        fingerprint,
        "prepare: making the pipelines earlier runs drew with, while the title starts"
    );
    *progress() = Some(Progress {
        total: work.len(),
        done: 0,
        stuck: 0,
        weight: work.iter().map(|w| weight_of(w.2)).sum(),
        done_weight: 0.0,
        finished: None,
    });
    make::set_preparing(true);
    for (_, p, _) in &work {
        // Workers up to cores less two.
        make::warm(p, cores.saturating_sub(2));
    }
    let mut open: Vec<usize> = (0..work.len()).collect();
    let mut journal_now: Vec<usize> = Vec::new();
    let (mut made, mut refused) = (0usize, 0usize);
    while !open.is_empty() {
        std::thread::sleep(Duration::from_millis(250));
        let (mut done_lines, mut stuck_lines) = (Vec::new(), Vec::new());
        let mut weight_now = 0.0;
        let mut in_flight = Vec::new();
        open.retain(|&i| {
            let (key, p, ms) = &work[i];
            match make::outcome(p) {
                Some(has) => {
                    if has { made += 1 } else { refused += 1 }
                    done_lines.push(format!("{fingerprint} {key}"));
                    weight_now += weight_of(*ms);
                    false
                }
                None => match make::making_for(p) {
                    Some(t) if t > deadline(*ms) => {
                        tracing::warn!(key, seconds = t.as_secs(), "prepare: a compile did not come back; left to the game");
                        stuck_lines.push(format!("{fingerprint} {key} did not come back in {} s", t.as_secs()));
                        weight_now += weight_of(*ms);
                        false
                    }
                    Some(_) => {
                        in_flight.push(i);
                        true
                    }
                    None => true,
                },
            }
        });
        append(&path(DONE_FILE), &done_lines);
        append(&path(STUCK_FILE), &stuck_lines);
        if in_flight != journal_now {
            let text: String = in_flight.iter().map(|&i| format!("{}\n", work[i].0)).collect();
            if let Err(e) = std::fs::write(path(JOURNAL_FILE), text) {
                tracing::warn!(error = %e, "prepare: the journal was not written");
            }
            journal_now = in_flight;
        }
        if let Some(p) = progress().as_mut() {
            p.done += done_lines.len() + stuck_lines.len();
            p.stuck += stuck_lines.len();
            p.done_weight += weight_now;
        }
    }
    let _ = std::fs::remove_file(path(JOURNAL_FILE));
    make::set_preparing(false);
    super::library::save(true);
    let stuck = progress().as_mut().map_or(0, |p| {
        p.finished = Some(Instant::now());
        p.stuck
    });
    tracing::info!(
        made,
        refused,
        stuck,
        seconds = format!("{:.1}", began.elapsed().as_secs_f64()),
        "prepare: the pipelines earlier runs drew with are made"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    /// Only what the book has and is not done, bad or stuck; the long ones
    /// first, longest first; the others in the list's order, once each.
    #[test]
    fn the_long_ones_go_first_and_the_done_are_left_out() {
        let used = keys(&["a", "b", "c", "d", "e", "b", "f"]);
        let in_book: HashSet<String> = keys(&["a", "b", "c", "d", "e"]).into_iter().collect();
        let skip: HashSet<String> = keys(&["c"]).into_iter().collect();
        let slow = |k: &str| match k {
            "d" => Some(4000),
            "e" => Some(47000),
            _ => None,
        };
        assert_eq!(pick(&used, &in_book, &skip, slow), keys(&["e", "d", "a", "b"]));
    }

    #[test]
    fn a_compile_is_given_a_minute_or_three_times_its_last() {
        assert_eq!(deadline(None), Duration::from_secs(60));
        assert_eq!(deadline(Some(4000)), Duration::from_secs(60));
        assert_eq!(deadline(Some(47_318)), Duration::from_millis(141_954));
    }

    #[test]
    fn keys_are_read_for_one_fingerprint_or_by_the_first_word() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("done.txt");
        std::fs::write(&file, "f1 a\nf2 b\nf1 c reason\n\nx\n").unwrap();
        assert_eq!(keys_in(&file, Some("f1")), keys(&["a", "c"]).into_iter().collect());
        assert_eq!(keys_in(&file, None), keys(&["f1", "f2", "x"]).into_iter().collect());
        assert!(keys_in(&dir.path().join("none"), None).is_empty());
    }

    /// The box: one line, the words on the left and the share on the
    /// right, over a bar filled with the expected time of what is done (not
    /// the count); full for two seconds at the end, then no box.
    #[test]
    fn the_box_shows_how_far_and_then_goes() {
        let began = Instant::now();
        let mut p = Progress {
            total: 200,
            done: 50,
            stuck: 0,
            weight: 120_000.0,
            done_weight: 30_000.0,
            finished: None,
        };
        let at = |s| began + Duration::from_secs(s);
        assert_eq!(p.shown(at(20)), Some(("PREPARING SHADERS    25%".to_owned(), 0.25)));
        // A quarter of them, but half the expected time: half full.
        let half = Progress { done_weight: 60_000.0, ..p.clone() }.shown(at(20)).unwrap();
        assert_eq!(half, ("PREPARING SHADERS    50%".to_owned(), 0.5));
        assert_eq!(half.0.len(), COLUMNS);
        (p.done, p.done_weight, p.finished) = (200, 120_000.0, Some(at(150)));
        assert_eq!(p.shown(at(151)), Some(("PREPARING SHADERS   100%".to_owned(), 1.0)));
        assert_eq!(p.shown(at(152)), None, "full for two seconds only");
    }

}
