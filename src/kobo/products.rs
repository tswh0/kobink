use serde_json::{json, Value};

use crate::{
    kobo::library::models::Entitlement,
    library::models::Book,
};

pub fn product(book: &Book) -> Value {
    let mut value =
        serde_json::to_value(Entitlement::new(book).BookMetadata).unwrap_or(Value::Null);

    if let Some(object) = value.as_object_mut() {
        object.insert("Id".to_string(), json!(book.id));
        object.insert("Prices".to_string(), json!([]));
        object.insert("Availability".to_string(), json!({ "IsAvailable": true }));
    }

    value
}

pub fn newest(books: &[Book], count: usize) -> Vec<Value> {
    let mut sorted: Vec<&Book> = books.iter().collect();
    sorted.sort_by(|left, right| right.modified.cmp(&left.modified));

    sorted.into_iter().take(count).map(product).collect()
}

pub fn waiting(books: &[Book], count: usize) -> Vec<Value> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() / 86_400)
        .unwrap_or_default();

    let mut sorted: Vec<&Book> = books.iter().collect();
    sorted.sort_by_key(|book| rotate(book.id.as_u128(), seed));

    sorted.into_iter().take(count).map(product).collect()
}

pub fn collecting(books: &[Book], count: usize) -> Vec<Value> {
    let mut started: Vec<(&str, f64)> = Vec::new();

    for book in books {
        let Some(series) = book.series.as_ref() else {
            continue;
        };

        match started.iter_mut().find(|(name, _)| series.name.as_str() == *name) {
            Some((_, number)) => {
                if series.number < *number {
                    *number = series.number;
                }
            }
            None => started.push((series.name.as_str(), series.number)),
        }
    }

    started.sort_by(|left, right| left.0.cmp(right.0));

    started
        .into_iter()
        .filter_map(|(name, number)| next_in_series(books, name, number))
        .take(count)
        .map(product)
        .collect()
}

pub fn nextread(books: &[Book], book: &Book) -> Option<Value> {
    let series = book.series.as_ref()?;

    next_in_series(books, &series.name, series.number).map(product)
}

fn next_in_series<'a>(books: &'a [Book], name: &str, after: f64) -> Option<&'a Book> {
    books
        .iter()
        .filter(|candidate| {
            candidate
                .series
                .as_ref()
                .is_some_and(|series| series.name == name && series.number > after)
        })
        .min_by(|left, right| {
            left.series
                .as_ref()
                .map(|series| series.number)
                .unwrap_or_default()
                .total_cmp(
                    &right
                        .series
                        .as_ref()
                        .map(|series| series.number)
                        .unwrap_or_default(),
                )
        })
}

fn rotate(id: u128, day: u64) -> u64 {
    (id as u64 ^ day).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}
