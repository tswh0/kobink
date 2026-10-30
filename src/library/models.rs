use epub::doc::EpubDoc;
use std::path::PathBuf;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    auth::{api_key, public_base},
    kobo::library::models::DownloadUrl,
};

#[derive(Debug, Clone)]
pub struct Book {
    pub id: Uuid,
    pub title: String,
    pub authors: Vec<String>,
    pub description: String,
    pub path: PathBuf,
    pub size: u64,
}

impl Book {
    pub fn format(&self) -> &'static str {
        if self.path.to_string_lossy().ends_with(".kepub.epub") {
            "KEPUB"
        } else {
            "EPUB"
        }
    }

    pub fn formats(&self) -> &'static [&'static str] {
        match self.format() {
            "KEPUB" => &["KEPUB"],
            _ => &["EPUB3", "EPUB"],
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
    pub path: String,
    pub books: Vec<Book>,
}

impl LibraryState {
    pub fn new(library_path: &str) -> Self {
        Self {
            path: library_path.to_string(),
            books: scan(library_path),
        }
    }

    pub fn find_book_by_id(&self, id: &str) -> Option<&Book> {
        self.books.iter().find(|book| book.id.to_string() == id)
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

pub fn scan(library_path: &str) -> Vec<Book> {
    let mut books = Vec::new();
    let mut unreadable = 0;

    for entry in WalkDir::new(library_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "epub"))
    {
        let epub = match EpubDoc::new(entry.path()) {
            Ok(epub) => epub,
            Err(error) => {
                unreadable += 1;
                tracing::warn!(
                    "failed to load epub at {}: {error:?}",
                    entry.path().display()
                );
                continue;
            }
        };

        let title = epub.mdata("title").unwrap_or("Untitled".to_string());

        // we can use metadata.get("creator") to get multiple authors
        let author = epub
            .mdata("creator")
            .unwrap_or("Unknown author".to_string());
        let description = epub.mdata("description").unwrap_or("".to_string());

        let filename = entry.file_name().to_string_lossy().to_string();
        let id = book_id(&format!("{title}|{author}|{filename}"));

        books.push(Book {
            id,
            title,
            authors: vec![author],
            description,
            path: entry.path().to_path_buf(),
            size: entry.metadata().map(|metadata| metadata.len()).unwrap_or(0),
        });
    }

    tracing::info!(
        "loaded {} books from {} ({} unreadable)",
        books.len(),
        library_path,
        unreadable
    );
    books
}
