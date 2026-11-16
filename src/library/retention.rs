use std::{
    fs,
    path::Path,
    sync::OnceLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tracing::info;

use super::models::TRASH;

const DEFAULT_DAYS: u64 = 30;
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

pub fn sweep(library_path: &str, covers_dir: &str) {
    let covers = days("COVER_CACHE_DAYS");
    let trash = days("TRASH_DAYS");

    announce(covers, trash);

    if covers > 0 {
        let removed = prune_files(Path::new(covers_dir), covers);

        if removed > 0 {
            info!("retention: removed {removed} cover(s) older than {covers} days");
        }
    }

    if trash > 0 {
        let removed = prune_trash(&Path::new(library_path).join(TRASH), trash);

        if removed > 0 {
            info!("retention: removed {removed} trash entr(y/ies) older than {trash} days");
        }
    }
}

fn announce(covers: u64, trash: u64) {
    static ANNOUNCED: OnceLock<()> = OnceLock::new();

    if ANNOUNCED.set(()).is_ok() {
        info!(
            "retention: covers {}, trash {}",
            policy(covers),
            policy(trash)
        );
    }
}

fn policy(days: u64) -> String {
    if days == 0 {
        "kept forever".to_string()
    } else {
        format!("after {days} days")
    }
}

fn days(name: &str) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(DEFAULT_DAYS)
}

fn cutoff(days: u64) -> Option<SystemTime> {
    SystemTime::now().checked_sub(Duration::from_secs(days * SECONDS_PER_DAY))
}

fn prune_files(dir: &Path, days: u64) -> usize {
    let Some(cutoff) = cutoff(days) else {
        return 0;
    };

    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };

    let mut removed = 0;

    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if metadata.is_file() && metadata.modified().is_ok_and(|modified| modified < cutoff) {
            if fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }

    removed
}

fn prune_trash(dir: &Path, days: u64) -> usize {
    let Some(cutoff) = cutoff(days) else {
        return 0;
    };

    let epoch = cutoff
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(u64::MAX);

    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };

    let mut removed = 0;

    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };

        // trash from before the flat layout is a directory per retire, named by its epoch
        let expired = if kind.is_dir() {
            match entry.file_name().to_string_lossy().parse::<u64>() {
                Ok(stamp) => stamp < epoch,
                Err(_) => modified_before(&entry, cutoff),
            }
        } else {
            modified_before(&entry, cutoff)
        };

        let gone = if kind.is_dir() {
            fs::remove_dir_all(entry.path())
        } else {
            fs::remove_file(entry.path())
        };

        if expired && gone.is_ok() {
            removed += 1;
        }
    }

    removed
}

fn modified_before(entry: &fs::DirEntry, cutoff: SystemTime) -> bool {
    entry
        .metadata()
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .is_some_and(|modified| modified < cutoff)
}
