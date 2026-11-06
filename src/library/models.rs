use epub::doc::EpubDoc;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    auth::{api_key, public_base},
    kobo::library::models::DownloadUrl,
};

#[derive(Debug, Clone)]
pub struct BookSeries {
    pub name: String,
    pub number: f64,
}

impl BookSeries {
    pub fn new(name: &str, number: f64) -> Option<Self> {
        let name = name.trim();

        if name.is_empty() {
            return None;
        }

        Some(Self {
            name: name.to_string(),
            number,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Book {
    pub id: Uuid,
    pub title: String,
    pub author: String,
    pub description: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub fixed_layout: bool,
    pub is_kepub: bool,
    pub series: Option<BookSeries>,
}

impl Book {
    pub fn format(&self) -> &'static str {
        if self.is_kepub {
            "KEPUB"
        } else {
            "EPUB"
        }
    }

    pub fn formats(&self) -> &'static [&'static str] {
        if self.fixed_layout {
            &["EPUB3FL"]
        } else {
            match self.format() {
                "KEPUB" => &["KEPUB"],
                _ => &["EPUB3", "EPUB"],
            }
        }
    }

    pub fn filename(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn download_urls(&self) -> Vec<DownloadUrl> {
        self.formats()
            .iter()
            .map(|format| self.download_url(format))
            .collect()
    }

    pub fn download_url(&self, format: &str) -> DownloadUrl {
        DownloadUrl {
            Format: format.to_string(),
            Size: self.size,
            Url: format!(
                "{}/{}/download/{book_id}/{book_format}/{filename}",
                public_base(),
                api_key(),
                book_id = self.id,
                book_format = format,
                filename = self.filename().replace(" ", "_")
            ),
            Platform: "Generic".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LibraryState {
    pub books: Arc<Vec<Book>>,
    pub version: u64,
}

impl LibraryState {
    pub fn new(library_path: &str) -> Self {
        let books = scan(library_path, &[]);
        let version = version(&books);

        Self {
            books: Arc::new(books),
            version,
        }
    }

    pub fn find_book_by_id(&self, id: &str) -> Option<&Book> {
        let id = id.parse::<Uuid>().ok()?;
        self.books.iter().find(|book| book.id == id)
    }
}

pub fn book_id(seed: &str) -> Uuid {
    const OFFSET_BASIS: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013b;

    let mut hash = OFFSET_BASIS;
    for byte in seed.as_bytes() {
        hash ^= *byte as u128;
        hash = hash.wrapping_mul(PRIME);
    }

    Uuid::from_u128(hash)
}

pub fn version(books: &[Book]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET_BASIS;
    for book in books {
        let size = book.size.to_le_bytes();
        let modified = book
            .modified
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
            .to_le_bytes();

        for bytes in [book.id.as_bytes(), size.as_slice(), modified.as_slice()] {
            for byte in bytes {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(PRIME);
            }
        }
    }

    hash
}

pub fn scan(library_path: &str, previous: &[Book]) -> Vec<Book> {
    let mut books = Vec::new();
    let mut unreadable = 0;

    let known: HashMap<&Path, &Book> = previous
        .iter()
        .map(|book| (book.path.as_path(), book))
        .collect();

    for entry in WalkDir::new(library_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "epub"))
    {
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let size = metadata.len();
        let modified = metadata.modified().ok();

        if let Some(book) = known.get(entry.path()) {
            if book.size == size && book.modified == modified {
                books.push((*book).clone());
                continue;
            }
        }

        let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            read_book(entry.path(), size, modified)
        }));

        match parsed {
            Ok(Some(book)) => books.push(book),
            Ok(None) => unreadable += 1,
            Err(_) => {
                unreadable += 1;
                tracing::warn!("epub at {} panicked while parsing", entry.path().display());
            }
        }
    }

    tracing::info!(
        "loaded {} books from {} ({} unreadable)",
        books.len(),
        library_path,
        unreadable
    );

    books
}

fn read_book(path: &Path, size: u64, modified: Option<SystemTime>) -> Option<Book> {
    let epub = match EpubDoc::new(path) {
        Ok(epub) => epub,
        Err(error) => {
            tracing::warn!("failed to load epub at {}: {error:?}", path.display());
            return None;
        }
    };

    let title = epub.get_title().unwrap_or_else(|| "Untitled".to_string());
    let author = epub
        .mdata("creator")
        .map(|item| item.value.clone())
        .unwrap_or_else(|| "Unknown author".to_string());
    let description = epub
        .mdata("description")
        .map(|item| item.value.clone())
        .unwrap_or_default();

    let fixed_layout = epub
        .mdata("rendition:layout")
        .is_some_and(|item| item.value.trim() == "pre-paginated");

    let filename = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let id = book_id(&format!("{title}|{author}|{filename}"));
    let series = series_of(&epub).or_else(|| series_from_title(&title));

    Some(Book {
        id,
        title,
        author,
        description,
        path: path.to_path_buf(),
        size,
        modified,
        fixed_layout,
        is_kepub: filename.ends_with(".kepub.epub"),
        series,
    })
}

fn series_of<R: std::io::Read + std::io::Seek>(epub: &EpubDoc<R>) -> Option<BookSeries> {
    if let Some(item) = epub.mdata("calibre:series") {
        let number = epub
            .mdata("calibre:series_index")
            .and_then(|index| index.value.trim().parse::<f64>().ok())
            .unwrap_or(1.0);

        if let Some(series) = BookSeries::new(&item.value, number) {
            return Some(series);
        }
    }

    for item in &epub.metadata {
        if item.property != "belongs-to-collection" {
            continue;
        }

        let kind = item.refinement("collection-type").map(|kind| kind.value.trim());
        if kind.is_some_and(|kind| kind != "series") {
            continue;
        }

        let number = item
            .refinement("group-position")
            .and_then(|position| position.value.trim().parse::<f64>().ok())
            .unwrap_or(1.0);

        if let Some(series) = BookSeries::new(&item.value, number) {
            return Some(series);
        }
    }

    None
}

fn series_from_title(title: &str) -> Option<BookSeries> {
    let trimmed = title.trim();
    let lower = trimmed.to_lowercase();

    for marker in ["volume", "vol.", "vol", "v"] {
        let Some(index) = lower.rfind(marker) else {
            continue;
        };

        if !trimmed.is_char_boundary(index) {
            continue;
        }

        let (head, tail) = trimmed.split_at(index);
        if !matches!(head.chars().last(), Some(' ') | Some('-') | Some('_')) {
            continue;
        }

        let rest = tail[marker.len()..]
            .trim_start()
            .trim_start_matches(['.', ',', ' ', '-', '_']);
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();

        if digits.is_empty() || digits.len() > 3 {
            continue;
        }

        let name = head.trim_end_matches([' ', '-', '_']);
        if name.is_empty() {
            continue;
        }

        let number = digits.parse::<f64>().ok()?;
        return BookSeries::new(name, number);
    }

    None
}
