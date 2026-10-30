use epub::doc::EpubDoc;
use std::path::PathBuf;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{auth::api_key, kobo::library::models::DownloadUrl};

const DEFAULT_PROTOCOL: &'static str = "https";
const DEFAULT_HOST: &'static str = "tickets.eggworld.me";

fn public_base() -> String {
    std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| format!("{}://{}", DEFAULT_PROTOCOL, DEFAULT_HOST))
        .trim_end_matches('/')
        .to_string()
}

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
    pub fn download_url(&self) -> DownloadUrl {
        DownloadUrl {
            Format: "EPUB".to_string(),
            Size: self.size,
            Url: format!(
                "{}/{}/download/{book_id}/{book_format}/{filename}",
                public_base(),
                api_key(),
                book_id = self.id,
                book_format = "EPUB",
                filename = self
                    .path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace(" ", "_")
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

pub fn scan(library_path: &str) -> Vec<Book> {
    let mut books = Vec::new();

    for entry in WalkDir::new(library_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "epub"))
    {
        let Ok(epub) = EpubDoc::new(entry.path()) else {
            tracing::info!("failed to load epub at {:?}", entry.path());
            continue;
        };

        let title = epub.mdata("title").unwrap_or("Untitled".to_string());

        // we can use metadata.get("creator") to get multiple authors
        let author = epub
            .mdata("creator")
            .unwrap_or("Unknown author".to_string());
        let description = epub.mdata("description").unwrap_or("".to_string());

        let mut seed = [0u8; 16];
        let seed_key = format!("{}{}", title, author);
        let seed_source = seed_key.as_bytes();
        let copy_len = std::cmp::min(seed.len(), seed_source.len());
        seed[..copy_len].copy_from_slice(&seed_source[..copy_len]);

        books.push(Book {
            id: Uuid::from_bytes(seed),
            title,
            authors: vec![author],
            description,
            path: entry.path().to_path_buf(),
            size: entry.metadata().unwrap().len(),
        });
    }

    tracing::info!("loaded {} books from {}", books.len(), library_path);
    books
}
