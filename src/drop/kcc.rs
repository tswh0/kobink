use std::{
    path::Path,
    process::Stdio,
    sync::OnceLock,
    time::Duration,
};

use tokio::{process::Command, time::timeout};

const CONVERT_TIMEOUT: Duration = Duration::from_secs(900);
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(20);
const LOOKUP_SCRIPT: &str = include_str!("author_lookup.py");

pub const DEFAULT_PROFILE: &str = "KoL";

pub const PROFILES: &[(&str, &str, &str)] = &[
    ("KoL", "Kobo Libra H2O / Libra 2", "Kobo"),
    ("KoLC", "Kobo Libra Colour", "Kobo"),
    ("KoC", "Kobo Clara HD / 2E / BW", "Kobo"),
    ("KoCC", "Kobo Clara Colour", "Kobo"),
    ("KoG", "Kobo Glo", "Kobo"),
    ("KoGHD", "Kobo Glo HD", "Kobo"),
    ("KoA", "Kobo Aura", "Kobo"),
    ("KoAHD", "Kobo Aura HD", "Kobo"),
    ("KoAH2O", "Kobo Aura H2O", "Kobo"),
    ("KoAO", "Kobo Aura ONE", "Kobo"),
    ("KoN", "Kobo Nia", "Kobo"),
    ("KoF", "Kobo Forma", "Kobo"),
    ("KoS", "Kobo Sage", "Kobo"),
    ("KoE", "Kobo Elipsa", "Kobo"),
    ("KoMT", "Kobo Mini / Touch", "Kobo"),
    ("KPW5", "Kindle Paperwhite 5", "Kindle"),
    ("KPW6", "Kindle Paperwhite 6", "Kindle"),
    ("KPW34", "Kindle Paperwhite 3 / 4 / Oasis", "Kindle"),
    ("KPW", "Kindle Paperwhite 1 / 2", "Kindle"),
    ("K11", "Kindle 11", "Kindle"),
    ("K810", "Kindle 8 / 10", "Kindle"),
    ("K34", "Kindle Keyboard / Touch", "Kindle"),
    ("K57", "Kindle 5 / 7", "Kindle"),
    ("KO", "Kindle Oasis 2 / 3", "Kindle"),
    ("KV", "Kindle Voyage", "Kindle"),
    ("KS", "Kindle Scribe 1 / 2", "Kindle"),
    ("KS3", "Kindle Scribe 3", "Kindle"),
    ("KCS", "Kindle Colorsoft", "Kindle"),
    ("KSCS", "Kindle Scribe Colorsoft", "Kindle"),
    ("Rmk1", "reMarkable 1", "reMarkable"),
    ("Rmk2", "reMarkable 2", "reMarkable"),
    ("RmkPP", "reMarkable Paper Pro", "reMarkable"),
    ("RmkPPMove", "reMarkable Paper Pro Move", "reMarkable"),
    ("OTHER", "Other", "Other"),
];

pub fn bin() -> String {
    std::env::var("KCC_BIN").unwrap_or_else(|_| "kcc-c2e".to_string())
}

pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();

    *AVAILABLE.get_or_init(|| {
        std::process::Command::new(bin())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    })
}

pub fn profile(code: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    PROFILES.iter().find(|(known, _, _)| *known == code)
}

pub fn extension(profile: &str) -> &'static str {
    if profile.starts_with("Ko") {
        ".kepub.epub"
    } else {
        ".epub"
    }
}

pub async fn convert(
    input: &Path,
    output: &Path,
    profile: &str,
    title: &str,
    author: Option<&str>,
) -> Result<(), String> {
    let mut command = Command::new(bin());

    command
        .arg("-p")
        .arg(profile)
        .arg("-m")
        .arg("-r")
        .arg("2")
        .arg("--coverfill")
        .arg("-f")
        .arg("EPUB")
        .arg("-t")
        .arg(title);

    if let Some(author) = author {
        command.arg("-a").arg(author);
    }

    command
        .arg("-o")
        .arg(output)
        .arg("--")
        .arg(input)
        .env("HOME", std::env::temp_dir())
        .stdin(Stdio::null())
        .kill_on_drop(true);

    let finished = match timeout(CONVERT_TIMEOUT, command.output()).await {
        Ok(Ok(finished)) => finished,
        Ok(Err(error)) => return Err(format!("cannot run {}: {error}", bin())),
        Err(_) => return Err("conversion timed out".to_string()),
    };

    if !finished.status.success() {
        let stderr = String::from_utf8_lossy(&finished.stderr);
        let last = stderr.lines().rev().find(|line| !line.trim().is_empty());

        return Err(format!(
            "kcc exited with {}: {}",
            finished.status,
            last.unwrap_or("no output").trim()
        ));
    }

    if !output.exists() {
        return Err(format!("kcc wrote nothing to {}", output.display()));
    }

    Ok(())
}

pub async fn author(series: &str, sample: &Path) -> Option<String> {
    let mut command = Command::new("python3");

    command
        .arg("-c")
        .arg(LOOKUP_SCRIPT)
        .arg(series)
        .arg(sample)
        .stdin(Stdio::null())
        .kill_on_drop(true);

    let finished = timeout(LOOKUP_TIMEOUT, command.output()).await.ok()?.ok()?;
    let name = String::from_utf8_lossy(&finished.stdout).trim().to_string();

    (!name.is_empty() && finished.status.success()).then_some(name)
}
