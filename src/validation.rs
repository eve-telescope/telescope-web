//! The request rules with Laravel's English messages and its semantics:
//! `required` stops at a missing, null or empty value; every other rule
//! skips a field the request did not send and, with `nullable`, a null
//! one, but judges a null that is not nullable; every failing rule adds
//! its message, in the order the rules are listed; the answer is 422
//! `{"message": "<first> (and N more errors)", "errors": {...}}`, whatever
//! the client accepts (FIXES.md).

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use regex::Regex;
use serde_json::{Map, Value, json};
use time::OffsetDateTime;

use crate::dates;
use crate::input::Input;

/// Every message of every field that failed, the fields in the order
/// their rules ran.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Errors(pub Vec<(String, Vec<String>)>);

impl Errors {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn add(&mut self, field: &str, message: String) {
        match self.0.iter_mut().find(|(name, _)| name == field) {
            Some((_, messages)) => messages.push(message),
            None => self.0.push((field.to_owned(), vec![message])),
        }
    }

    pub fn has(&self, field: &str) -> bool {
        self.0.iter().any(|(name, _)| name == field)
    }

    /// Laravel's JSON answer of a failed validation.
    pub fn json(&self) -> Value {
        let mut all = self.0.iter().flat_map(|(_, messages)| messages);
        let first = all.next().cloned().unwrap_or_default();
        let message = match all.count() {
            0 => first,
            1 => format!("{first} (and 1 more error)"),
            others => format!("{first} (and {others} more errors)"),
        };
        let errors: Map<String, Value> = self
            .0
            .iter()
            .map(|(field, messages)| (field.clone(), json!(messages)))
            .collect();
        json!({ "message": message, "errors": errors })
    }

    pub fn response(&self) -> Response {
        (StatusCode::UNPROCESSABLE_ENTITY, Json(self.json())).into_response()
    }
}

/// A field's name in messages: `entity_type` reads "entity type"; a
/// nested one (`pilots.1`) stays as it is.
pub fn attribute(field: &str) -> String {
    if field.contains('.') {
        field.to_owned()
    } else {
        field.replace('_', " ")
    }
}

pub struct Validator<'a> {
    fields: &'a Map<String, Value>,
    errors: Errors,
    now: OffsetDateTime,
}

/// What `min` and `max` measure, from the field's rules.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    String,
    Numeric,
    Array,
}

/// One field under validation.
pub struct Field<'v, 'a> {
    validator: &'v mut Validator<'a>,
    name: String,
    /// `None` when the request did not send the field.
    value: Option<&'a Value>,
    nullable: bool,
    /// `sometimes` met a missing field, or `required` failed.
    stopped: bool,
    kind: Kind,
}

impl<'a> Validator<'a> {
    pub fn new(input: &'a Input, now: OffsetDateTime) -> Self {
        Self {
            fields: &input.fields,
            errors: Errors::default(),
            now,
        }
    }

    pub fn field<'v>(&'v mut self, name: &str) -> Field<'v, 'a> {
        let value = self.fields.get(name);
        Field {
            validator: self,
            name: name.to_owned(),
            value,
            nullable: false,
            stopped: false,
            kind: Kind::String,
        }
    }

    /// The rules of every item of a list (`pilots.*`), as `pilots.0`,
    /// `pilots.1`, ... A field that is no list has no items.
    pub fn each(&mut self, name: &str, rules: impl Fn(Field<'_, 'a>)) {
        let items: Vec<(String, &'a Value)> = match self.fields.get(name) {
            Some(Value::Array(items)) => items
                .iter()
                .enumerate()
                .map(|(index, item)| (index.to_string(), item))
                .collect(),
            Some(Value::Object(items)) => items
                .iter()
                .map(|(key, item)| (key.clone(), item))
                .collect(),
            _ => return,
        };
        for (key, item) in items {
            let field = Field {
                name: format!("{name}.{key}"),
                value: Some(item),
                nullable: false,
                stopped: false,
                kind: Kind::String,
                validator: self,
            };
            rules(field);
        }
    }

    pub fn has_error(&self, field: &str) -> bool {
        self.errors.has(field)
    }

    /// The 422 answer when any rule failed.
    pub fn finish(self) -> Result<(), Response> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.response())
        }
    }
}

/// PHP's `is_numeric` for a JSON value.
fn numeric(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => {
            let text = text.trim_start();
            text.parse::<f64>().ok().filter(|_| {
                !text.is_empty()
                    && text
                        .chars()
                        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
            })
        }
        _ => None,
    }
}

/// PHP's string conversion, for `in` and string lengths.
fn php_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(true) => Some("1".to_owned()),
        Value::Bool(false) | Value::Null => Some(String::new()),
        _ => None,
    }
}

/// The value of an integer field: a whole JSON number or the digits PHP's
/// `filter_var(FILTER_VALIDATE_INT)` accepts (no leading zeros).
pub fn integer(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64().or_else(|| {
            number
                .as_f64()
                .filter(|float| float.fract() == 0.0 && float.abs() < 9.0e15)
                .map(|float| float as i64)
        }),
        Value::String(text) => {
            let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
            (!digits.is_empty()
                && digits.chars().all(|c| c.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0')))
            .then(|| text.parse().ok())
            .flatten()
        }
        _ => None,
    }
}

impl Field<'_, '_> {
    fn attribute(&self) -> String {
        attribute(&self.name)
    }

    fn fail(&mut self, message: String) {
        self.validator.errors.add(&self.name, message);
    }

    /// A rule that is not implicit: it judges a value only when the field
    /// was sent, is not an empty string, and is not a null that may be
    /// null.
    fn check(
        mut self,
        passes: impl FnOnce(&Value, &Self) -> bool,
        message: impl FnOnce(&str, Kind) -> String,
    ) -> Self {
        let Some(value) = self.value else {
            return self;
        };
        if self.stopped
            || value.as_str().is_some_and(|text| text.trim().is_empty())
            || (self.nullable && value.is_null())
        {
            return self;
        }
        if !passes(value, &self) {
            let message = message(&self.attribute(), self.kind);
            self.fail(message);
        }
        self
    }

    /// `required`: sent, not null, not empty. Stops the field's other
    /// rules when it fails.
    pub fn required(mut self) -> Self {
        let missing = match self.value {
            None | Some(Value::Null) => true,
            Some(Value::String(text)) => text.trim().is_empty(),
            Some(Value::Array(items)) => items.is_empty(),
            Some(Value::Object(items)) => items.is_empty(),
            Some(_) => false,
        };
        if !self.stopped && missing {
            let message = format!("The {} field is required.", self.attribute());
            self.fail(message);
            self.stopped = true;
        }
        self
    }

    /// `sometimes`: the rules only apply when the field was sent.
    pub fn sometimes(mut self) -> Self {
        if self.value.is_none() {
            self.stopped = true;
        }
        self
    }

    /// `nullable`: null passes every rule.
    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    pub fn string(self) -> Self {
        self.check(
            |value, _| value.is_string(),
            |a, _| format!("The {a} field must be a string."),
        )
    }

    pub fn array(mut self) -> Self {
        self.kind = Kind::Array;
        self.check(
            |value, _| value.is_array() || value.is_object(),
            |a, _| format!("The {a} field must be an array."),
        )
    }

    pub fn integer(mut self) -> Self {
        self.kind = Kind::Numeric;
        self.check(
            |value, _| integer(value).is_some(),
            |a, _| format!("The {a} field must be an integer."),
        )
    }

    /// The size `min` and `max` compare: a number for a numeric field, a
    /// count for a list, characters otherwise.
    fn size(value: &Value, kind: Kind) -> Option<f64> {
        if kind == Kind::Numeric
            && let Some(number) = numeric(value)
        {
            return Some(number);
        }
        match value {
            Value::Array(items) => Some(items.len() as f64),
            Value::Object(items) => Some(items.len() as f64),
            other => php_string(other).map(|text| text.chars().count() as f64),
        }
    }

    pub fn min(self, min: usize) -> Self {
        self.check(
            |value, field| Self::size(value, field.kind).is_none_or(|size| size >= min as f64),
            |a, kind| match kind {
                Kind::Array => format!("The {a} field must have at least {min} items."),
                Kind::Numeric => format!("The {a} field must be at least {min}."),
                Kind::String => format!("The {a} field must be at least {min} characters."),
            },
        )
    }

    pub fn max(self, max: usize) -> Self {
        self.check(
            |value, field| Self::size(value, field.kind).is_none_or(|size| size <= max as f64),
            |a, kind| match kind {
                Kind::Array => format!("The {a} field must not have more than {max} items."),
                Kind::Numeric => format!("The {a} field must not be greater than {max}."),
                Kind::String => {
                    format!("The {a} field must not be greater than {max} characters.")
                }
            },
        )
    }

    /// `in:...`: the value as text is one of the list.
    pub fn one_of(self, allowed: &[&str]) -> Self {
        self.check(
            |value, _| {
                !value.is_array()
                    && !value.is_object()
                    && php_string(value).is_some_and(|text| allowed.contains(&text.as_str()))
            },
            |a, _| format!("The selected {a} is invalid."),
        )
    }

    pub fn regex(self, pattern: &Regex) -> Self {
        self.check(
            |value, _| match value {
                Value::String(text) => pattern.is_match(text),
                Value::Number(number) => pattern.is_match(&number.to_string()),
                _ => false,
            },
            |a, _| format!("The {a} field format is invalid."),
        )
    }

    /// `date`: an ISO 8601 date of a day that exists (FIXES.md).
    pub fn date(self) -> Self {
        self.check(
            |value, _| value.as_str().and_then(dates::parse).is_some(),
            |a, _| format!("The {a} field must be a valid date."),
        )
    }

    /// `after:now`, compared like PHP's `strtotime`.
    pub fn after_now(self) -> Self {
        let now = self.validator.now;
        self.check(
            |value, _| {
                value
                    .as_str()
                    .and_then(|text| dates::parse_lenient(text, now))
                    .is_some_and(|moment| moment > now)
            },
            |a, _| format!("The {a} field must be a date after now."),
        )
    }

    /// Whether this field has an error so far.
    pub fn failed(&self) -> bool {
        self.validator.has_error(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn input(value: Value) -> Input {
        Input {
            fields: value.as_object().cloned().expect("an object"),
        }
    }

    #[test]
    fn counts_the_other_errors() {
        let input = input(json!({ "name": null, "label": 5 }));
        let mut validator = Validator::new(&input, datetime!(2026-10-13 08:30:00 UTC));
        validator.field("name").required().string().max(255);
        validator.field("label").sometimes().string().max(2);
        validator.field("absent").string();
        let errors = validator.finish().expect_err("errors");
        assert_eq!(errors.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn names_the_attributes_like_laravel() {
        assert_eq!(attribute("entity_type"), "entity type");
        assert_eq!(attribute("entity_ids.0"), "entity_ids.0");
    }

    #[test]
    fn reads_integers_like_filter_var() {
        assert_eq!(integer(&json!(12345)), Some(12345));
        assert_eq!(integer(&json!(12345.0)), Some(12345));
        assert_eq!(integer(&json!("-12")), Some(-12));
        assert_eq!(integer(&json!("012")), None);
        assert_eq!(integer(&json!(1.5)), None);
        assert_eq!(integer(&json!(true)), None);
        assert_eq!(integer(&json!("1e3")), None);
    }

    #[test]
    fn a_null_that_may_not_be_null_is_judged() {
        let input = input(json!({ "permission": null }));
        let mut validator = Validator::new(&input, datetime!(2026-10-13 08:30:00 UTC));
        validator
            .field("permission")
            .sometimes()
            .string()
            .one_of(&["viewer", "member", "manager"]);
        assert_eq!(
            validator.errors.json(),
            json!({
                "message": "The permission field must be a string. (and 1 more error)",
                "errors": { "permission": [
                    "The permission field must be a string.",
                    "The selected permission is invalid.",
                ] },
            })
        );
    }
}
