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
    pub fn new(book: &Book) -> Self {
        Self {
            NewEntitlement: Entitlement::new(book),
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
        }
    }
}
