use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{atomic::{AtomicU64, Ordering}, Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

pub const DIR: &str = ".kobink-progress";

static GENERATION: AtomicU64 = AtomicU64::new(0);
struct Snapshot {
    generation: u64,
    dir: PathBuf,
    map: Arc<HashMap<String, Progress>>,
}

static SNAPSHOT: Mutex<Option<Snapshot>> = Mutex::new(None);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    pub percent: f64,
    pub chapter: String,
    pub chapter_percent: f64,
    pub page: u32,
    pub status: String,
    pub started: u32,
    pub span: String,
    pub updated: u64,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

pub fn dir(library_path: &str) -> PathBuf {
    Path::new(library_path).join(DIR)
}

pub fn load(library_path: &str, id: &str) -> Option<Progress> {
    let bytes = fs::read(dir(library_path).join(format!("{id}.json"))).ok()?;

    serde_json::from_slice(&bytes).ok()
}

pub fn all(library_path: &str) -> HashMap<String, Progress> {
    (*snapshot(library_path)).clone()
}

pub fn snapshot(library_path: &str) -> Arc<HashMap<String, Progress>> {
    let dir = dir(library_path);
    let generation = GENERATION.load(Ordering::Relaxed);

    {
        let cache = SNAPSHOT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(snapshot) = cache.as_ref() {
            if snapshot.generation == generation && snapshot.dir == dir {
                return snapshot.map.clone();
            }
        }
    }

    let map = Arc::new(read_all(&dir));
    let mut cache = SNAPSHOT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *cache = Some(Snapshot {
        generation,
        dir,
        map: map.clone(),
    });

    map
}

fn read_all(dir: &Path) -> HashMap<String, Progress> {
    let Ok(entries) = fs::read_dir(dir) else {
        return HashMap::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".json")?.to_string();
            let bytes = fs::read(entry.path()).ok()?;
            let progress = serde_json::from_slice(&bytes).ok()?;

            Some((id, progress))
        })
        .collect()
}

pub fn save(library_path: &str, id: &str, progress: &mut Progress) -> std::io::Result<()> {
    let dir = dir(library_path);
    fs::create_dir_all(&dir)?;

    progress.updated = now();

    if progress.status.is_empty() {
        progress.status = "Reading".to_string();
    }

    if progress.started == 0 {
        progress.started = 1;
    }

    let body = serde_json::to_vec(progress).map_err(std::io::Error::other)?;
    let target = dir.join(format!("{id}.json"));
    let temp = dir.join(format!("{id}.json.tmp"));

    fs::write(&temp, body)?;
    fs::rename(&temp, &target)?;
    bump(progress.updated);

    Ok(())
}

pub fn merge(stored: Option<Progress>, mut incoming: Progress) -> Progress {
    let Some(mut stored) = stored else {
        return settle(incoming);
    };

    incoming.started = stored.started.max(incoming.started);

    match incoming.status.as_str() {
        "ReadyToRead" => {
            stored.percent = 0.0;
            stored.chapter.clear();
            stored.chapter_percent = 0.0;
            stored.page = 0;
            stored.span.clear();
            stored.status = "ReadyToRead".to_string();

            stored
        }
        "Finished" => {
            incoming.percent = 100.0;
            incoming.chapter_percent = 100.0;

            if incoming.chapter.is_empty() {
                incoming.chapter = stored.chapter.clone();
            }

            settle(incoming)
        }
        _ if incoming.percent >= stored.percent => settle(incoming),
        _ => settle(stored),
    }
}

fn settle(mut progress: Progress) -> Progress {
    if progress.status != "ReadyToRead" {
        if progress.percent >= 99.5 {
            progress.status = "Finished".to_string();
        } else if progress.status == "Finished" {
            progress.status = "Reading".to_string();
        }
    }

    progress
}

pub fn remove(library_path: &str, id: &str) {
    if fs::remove_file(dir(library_path).join(format!("{id}.json"))).is_ok() {
        bump(now());
    }
}

pub fn prune(library_path: &str, keep: &HashSet<String>) -> usize {
    let Ok(entries) = fs::read_dir(dir(library_path)) else {
        return 0;
    };

    let mut removed = 0;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };

        if !keep.contains(id) && fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }

    if removed > 0 {
        bump(now());
    }

    removed
}

pub fn init(library_path: &str) {
    let newest = all(library_path)
        .values()
        .map(|progress| progress.updated)
        .max()
        .unwrap_or_default();

    bump(newest);
}

pub fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

fn bump(updated: u64) {
    let floor = updated.max(now());
    let mut current = GENERATION.load(Ordering::Relaxed);

    loop {
        let next = floor.max(current + 1);

        match GENERATION.compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }

    *SNAPSHOT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}
