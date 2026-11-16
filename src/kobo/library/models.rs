#![allow(non_snake_case)]

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::library::models::{book_id, Book};

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncEvent {
    pub NewEntitlement: Entitlement,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entitlement {
    pub BookEntitlement: BookEntitlement,
    pub BookMetadata: BookMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ReadingState: Option<ReadingState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadingState {
    pub Created: String,
    pub CurrentBookmark: Bookmark,
    pub EntitlementId: Uuid,
    pub LastModified: String,
    pub PriorityTimestamp: String,
    pub Statistics: ReadingStatistics,
    pub StatusInfo: StatusInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmark {
    pub ContentSourceProgressPercent: f64,
    pub LastModified: String,
    pub Location: ReadingLocation,
    pub ProgressPercent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadingLocation {
    pub Source: String,
    #[serde(rename = "Type")]
    pub kind: String,
    #[serde(default)]
    pub Value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadingStatistics {
    pub LastModified: String,
    pub RemainingTimeMinutes: u32,
    pub SpentReadingMinutes: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusInfo {
    pub LastModified: String,
    pub Status: String,
    pub TimesStartedReading: u32,
}

#[derive(Debug, Serialize)]
pub struct ReadingStates {
    pub ReadingStates: Vec<ReadingState>,
}

impl ReadingState {
    pub fn new(book: Uuid, progress: &crate::library::progress::Progress) -> Self {
        let modified = stamp_secs(progress.updated);
        let status = if progress.status.is_empty() {
            "Reading"
        } else {
            progress.status.as_str()
        };

        Self {
            Created: modified.clone(),
            CurrentBookmark: Bookmark {
                ContentSourceProgressPercent: progress.chapter_percent,
                LastModified: modified.clone(),
                Location: ReadingLocation {
                    Source: progress.chapter.clone(),
                    kind: "KoboSpan".to_string(),
                    Value: progress.span.clone(),
                },
                ProgressPercent: progress.percent,
            },
            EntitlementId: book,
            LastModified: modified.clone(),
            PriorityTimestamp: modified.clone(),
            Statistics: ReadingStatistics {
                LastModified: modified.clone(),
                RemainingTimeMinutes: 0,
                SpentReadingMinutes: 0,
            },
            StatusInfo: StatusInfo {
                LastModified: modified,
                Status: status.to_string(),
                TimesStartedReading: progress.started.max(1),
            },
        }
    }
}

fn stamp_secs(seconds: u64) -> String {
    DateTime::<Utc>::from_timestamp(seconds as i64, 0)
        .unwrap_or_else(Utc::now)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookEntitlement {
    pub Accessibility: String,
    pub ActivePeriod: ActivePeriod,
    pub Created: String,
    pub CrossRevisionId: Uuid,
    pub Id: Uuid,
    pub IsRemoved: bool,
    pub IsHiddenFromArchive: bool,
    pub IsLocked: bool,
    pub LastModified: String,
    pub OriginCategory: String,
    pub RevisionId: Uuid,
    pub Status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivePeriod {
    pub From: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookMetadata {
    pub Categories: Vec<String>,
    pub Contributors: Vec<String>,
    pub ContributorRoles: Vec<ContributorRole>,
    pub CoverImageId: Uuid,
    pub CrossRevisionId: Uuid,
    pub CurrentDisplayPrice: CurrentDisplayPrice,
    pub CurrentLoveDisplayPrice: CurrentLoveDisplayPrice,
    pub Description: String,
    pub DownloadUrls: Vec<DownloadUrl>,
    pub EntitlementId: Uuid,
    pub ExternalIds: Vec<ExternalId>,
    pub Genre: String,
    pub IsEligibleForKoboLove: bool,
    pub IsInternetArchive: bool,
    pub IsPreOrder: bool,
    pub IsSocialEnabled: bool,
    pub Language: String,
    pub PhoneticPronunciations: PhoneticPronunciations,
    pub PublicationDate: String,
    pub Publisher: Publisher,
    pub RevisionId: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Series: Option<Series>,
    pub Title: String,
    pub WorkId: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContributorRole {
    pub Name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrentDisplayPrice {
    pub CurrencyCode: String,
    pub TotalAmount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrentLoveDisplayPrice {
    pub TotalAmount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadUrl {
    pub Format: String,
    pub Size: u64,
    pub Url: String,
    pub Platform: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalId {}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhoneticPronunciations {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Publisher {
    pub Imprint: String,
    pub Name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Series {
    pub Name: String,
    pub Number: u64,
    pub NumberFloat: f64,
    pub Id: Uuid,
}

fn stamp(time: Option<SystemTime>) -> String {
    let time = time.map(DateTime::<Utc>::from).unwrap_or_else(Utc::now);

    time.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

impl SyncEvent {
    pub fn new(book: &Book, state: Option<ReadingState>) -> Self {
        Self {
            NewEntitlement: Entitlement::new(book).with_state(state),
        }
    }
}

impl Entitlement {
    pub fn new(book: &Book) -> Self {
        let mut entitlement = Self::empty(book.id, &stamp(book.modified));
        entitlement.BookMetadata.Title = book.title.clone();
        entitlement.BookMetadata.Description = book.description.clone();
        entitlement.BookMetadata.DownloadUrls = book.download_urls();
        entitlement.BookMetadata.Contributors = vec![book.author.clone()];
        entitlement.BookMetadata.ContributorRoles = vec![ContributorRole {
            Name: book.author.clone(),
        }];

        if let Some(series) = &book.series {
            entitlement.BookMetadata.Series = Some(Series {
                Name: series.name.clone(),
                Number: series.number.round() as u64,
                NumberFloat: series.number,
                Id: book_id(&format!("series|{}", series.name)),
            });
        }

        entitlement
    }

    fn empty(uuid: Uuid, stamp: &str) -> Self {

        Self {
            BookEntitlement: BookEntitlement {
                Accessibility: "Full".to_string(),
                ActivePeriod: ActivePeriod {
                    From: stamp.to_string(),
                },
                Created: stamp.to_string(),
                CrossRevisionId: uuid,
                Id: uuid,
                IsRemoved: false,
                IsHiddenFromArchive: false,
                IsLocked: false,
                LastModified: stamp.to_string(),
                OriginCategory: "Imported".to_string(),
                RevisionId: uuid,
                Status: "Active".to_string(),
            },
            BookMetadata: BookMetadata {
                Categories: vec!["00000000-0000-0000-000000000001".to_string()],
                Contributors: vec![],
                ContributorRoles: vec![],
                CoverImageId: uuid,
                CrossRevisionId: uuid,
                CurrentDisplayPrice: CurrentDisplayPrice {
                    CurrencyCode: "USD".to_string(),
                    TotalAmount: 0,
                },
                CurrentLoveDisplayPrice: CurrentLoveDisplayPrice { TotalAmount: 0 },
                Description: "".to_string(),
                DownloadUrls: vec![],
                EntitlementId: uuid,
                ExternalIds: vec![],
                Genre: "00000000-0000-0000-000000000001".to_string(),
                IsEligibleForKoboLove: false,
                IsInternetArchive: false,
                IsPreOrder: false,
                IsSocialEnabled: true,
                Language: "en".to_string(),
                PhoneticPronunciations: PhoneticPronunciations {},
                PublicationDate: stamp.to_string(),
                Publisher: Publisher {
                    Imprint: "".to_string(),
                    Name: "".to_string(),
                },
                RevisionId: uuid,
                Series: None,
                Title: "".to_string(),
                WorkId: uuid,
            },
            ReadingState: None,
        }
    }

    pub fn with_state(mut self, state: Option<ReadingState>) -> Self {
        self.ReadingState = state;
        self
    }
}
