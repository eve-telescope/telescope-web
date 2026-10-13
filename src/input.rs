//! The request input as validation sees it: the query string and the body
//! (JSON or a form) merged into one tree, bracket names (`entity_ids[]`)
//! nested (`x[a][]=1`), every string trimmed and empty strings turned
//! into null. A body that does not parse is no input.

use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value};

/// Invisible characters trimmed on top of whitespace and NUL.
const INVISIBLE_CHARACTERS: &[char] = &[
    '\u{00AD}',
    '\u{034F}',
    '\u{061C}',
    '\u{115F}',
    '\u{1160}',
    '\u{17B4}',
    '\u{17B5}',
    '\u{180E}',
    '\u{200B}',
    '\u{200C}',
    '\u{200D}',
    '\u{200E}',
    '\u{200F}',
    '\u{2060}',
    '\u{2061}',
    '\u{2062}',
    '\u{2063}',
    '\u{2064}',
    '\u{2065}',
    '\u{206A}',
    '\u{206B}',
    '\u{206C}',
    '\u{206D}',
    '\u{206E}',
    '\u{206F}',
    '\u{2800}',
    '\u{3164}',
    '\u{FEFF}',
    '\u{FFA0}',
    '\u{1D159}',
    '\u{1D173}',
    '\u{1D174}',
    '\u{1D175}',
    '\u{1D176}',
    '\u{1D177}',
    '\u{1D178}',
    '\u{1D179}',
    '\u{1D17A}',
    '\u{E0020}',
];

/// Strips Unicode whitespace, NUL and the invisible characters from both
/// ends.
pub fn trim(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || c == '\0' || INVISIBLE_CHARACTERS.contains(&c))
}

/// The merged, cleaned input of one request.
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub fields: Map<String, Value>,
}

impl Input {
    /// The value of a field; `None` when the request did not send it.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    /// Whether the request sent the field (null included).
    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// The value as a string, when it is one.
    pub fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    /// The query string alone, cleaned like a body.
    pub fn from_query(query: Option<&str>) -> Self {
        let mut fields = Map::new();
        if let Some(query) = query {
            parse_urlencoded(query.as_bytes(), &mut fields);
        }
        Self {
            fields: clean_map(fields),
        }
    }

    /// A body and the query: the body's fields win, the query fills in
    /// the rest, as `$request->all()` merged them.
    pub fn from_parts(query: Option<&str>, content_type: &str, body: &[u8]) -> Self {
        let mut fields = Map::new();
        let content_type = content_type.to_ascii_lowercase();
        if content_type.contains("json") {
            if let Ok(Value::Object(object)) = serde_json::from_slice::<Value>(body) {
                fields = object;
            }
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            parse_urlencoded(body, &mut fields);
        }
        if let Some(query) = query {
            let mut from_query = Map::new();
            parse_urlencoded(query.as_bytes(), &mut from_query);
            for (key, value) in from_query {
                fields.entry(key).or_insert(value);
            }
        }
        Self {
            fields: clean_map(fields),
        }
    }
}

impl<S: Send + Sync> FromRequest<S> for Input {
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let query = request.uri().query().map(str::to_owned);
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        let body = Bytes::from_request(request, state)
            .await
            .map_err(IntoResponse::into_response)?;
        Ok(Self::from_parts(query.as_deref(), &content_type, &body))
    }
}

/// `a=1&ids[]=2&ids[]=3&x[k]=4` into nested JSON: `[key]` nests an
/// object, `[]` appends to an array.
fn parse_urlencoded(bytes: &[u8], fields: &mut Map<String, Value>) {
    for (key, value) in form_urlencoded::parse(bytes) {
        insert_bracketed(fields, &key, Value::String(value.into_owned()));
    }
}

fn insert_bracketed(fields: &mut Map<String, Value>, name: &str, value: Value) {
    let (base, rest) = match name.find('[') {
        Some(index) if name.ends_with(']') => (&name[..index], &name[index..]),
        _ => (name, ""),
    };
    if base.is_empty() {
        return;
    }
    let segments: Vec<&str> = if rest.is_empty() {
        Vec::new()
    } else {
        rest[1..rest.len() - 1].split("][").collect()
    };
    let slot = fields.entry(base.to_owned()).or_insert(Value::Null);
    insert_segments(slot, &segments, value);
}

fn insert_segments(slot: &mut Value, segments: &[&str], value: Value) {
    let Some((first, rest)) = segments.split_first() else {
        *slot = value;
        return;
    };
    // `x[0]=a&x[1]=b` is a list too.
    let index = first.parse::<usize>().ok();
    if first.is_empty() || index.is_some_and(|index| slot.as_array().map_or(0, Vec::len) == index) {
        if !slot.is_array() {
            *slot = Value::Array(Vec::new());
        }
        let items = slot.as_array_mut().expect("an array");
        items.push(Value::Null);
        let last = items.last_mut().expect("just pushed");
        insert_segments(last, rest, value);
        return;
    }
    if let Value::Array(items) = slot {
        // A keyed field after list items: the object keeps both.
        let object: Map<String, Value> = items
            .drain(..)
            .enumerate()
            .map(|(index, item)| (index.to_string(), item))
            .collect();
        *slot = Value::Object(object);
    }
    if !slot.is_object() {
        *slot = Value::Object(Map::new());
    }
    let child = slot
        .as_object_mut()
        .expect("an object")
        .entry((*first).to_owned())
        .or_insert(Value::Null);
    insert_segments(child, rest, value);
}

fn clean_map(fields: Map<String, Value>) -> Map<String, Value> {
    fields
        .into_iter()
        .map(|(key, value)| (key, clean(value)))
        .collect()
}

fn clean(value: Value) -> Value {
    match value {
        Value::String(text) => {
            let text = trim(&text);
            if text.is_empty() {
                Value::Null
            } else {
                Value::String(text.to_owned())
            }
        }
        Value::Array(items) => Value::Array(items.into_iter().map(clean).collect()),
        Value::Object(fields) => Value::Object(clean_map(fields)),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn brackets_nest_like_parse_str() {
        let input = Input::from_query(Some(
            "entity_ids[]=1&entity_ids[]=&entity_ids%5B%5D=%203%20&x%5B0%5D=a&x%5B1%5D=b&plain=1",
        ));
        assert_eq!(
            Value::Object(input.fields),
            json!({
                "entity_ids": ["1", null, "3"],
                "x": ["a", "b"],
                "plain": "1",
            })
        );
    }

    #[test]
    fn the_body_wins_over_the_query() {
        let input = Input::from_parts(
            Some("name=query&page=2"),
            "application/json",
            "{\"name\":\"  body  \",\"notes\":\"\\u200b\"}".as_bytes(),
        );
        assert_eq!(
            Value::Object(input.fields),
            json!({ "name": "body", "notes": null, "page": "2" })
        );
    }

    #[test]
    fn a_broken_body_is_no_input() {
        let input = Input::from_parts(None, "application/json", b"{nope");
        assert!(input.fields.is_empty());
        let input = Input::from_parts(None, "application/json", b"[1,2]");
        assert!(input.fields.is_empty());
    }
}
