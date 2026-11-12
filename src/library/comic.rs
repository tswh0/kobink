use std::{
    fs::File,
    io::Read,
    path::Path,
};

use zip::ZipArchive;

const INFO: &str = "comicinfo.xml";
const INFO_LIMIT: u64 = 1024 * 1024;
const IMAGES: [&str; 6] = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];

pub fn open(path: &Path) -> Option<ZipArchive<File>> {
    ZipArchive::new(File::open(path).ok()?).ok()
}

pub fn author(archive: &mut ZipArchive<File>) -> Option<String> {
    let name = archive
        .file_names()
        .find(|name| name.to_ascii_lowercase().ends_with(INFO))?
        .to_string();

    let mut xml = String::new();
    archive
        .by_name(&name)
        .ok()?
        .take(INFO_LIMIT)
        .read_to_string(&mut xml)
        .ok()?;

    tag(&xml, "Writer").or_else(|| tag(&xml, "Author"))
}

pub fn cover(archive: &mut ZipArchive<File>, limit: u64) -> Option<Vec<u8>> {
    let name = page(archive)?;
    let mut data = Vec::new();

    archive
        .by_name(&name)
        .ok()?
        .take(limit)
        .read_to_end(&mut data)
        .ok()?;

    (!data.is_empty()).then_some(data)
}

fn page(archive: &ZipArchive<File>) -> Option<String> {
    let mut pages: Vec<&str> = archive.file_names().filter(|name| image(name)).collect();
    pages.sort_unstable();

    pages.first().map(|name| name.to_string())
}

fn image(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or_default();

    !lower.starts_with("__macosx/")
        && !file.starts_with('.')
        && lower
            .rsplit('.')
            .next()
            .is_some_and(|extension| IMAGES.contains(&extension))
}

fn tag(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let value = xml[start..end].trim();

    (!value.is_empty()).then(|| value.to_string())
}
