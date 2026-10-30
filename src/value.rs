//! Values: what a slot reads and a record holds.

use std::fmt;

use crate::record::Record;

/// A value, as typed by the sigil it was read with.
///
/// Exhaustive on purpose: a collapse matches on it, and a new variant
/// should break those matches.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `[abc]`, and members with no sigil.
    Text(String),
    /// `[#]`, kept as written.
    Number(String),
    /// `[31]`.
    Date(Date),
    /// `[-]`.
    Link(String),
    /// `[_]`.
    Bool(bool),
    /// `[$]`, kept as written.
    Currency(String),
    /// Trusted markup: a collapsed fragment or a rendered Markdown body.
    Html(Html),
    /// Markdown source, from a member whose value is a literal block. Its
    /// slots are not applied.
    Markdown(String),
    /// A source for `[List]`.
    List(Vec<Record>),
    /// A record, such as an instance's `this`.
    Record(Record),
}

impl Value {
    /// The variant's name, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Text(_) => "text",
            Value::Number(_) => "a number",
            Value::Date(_) => "a date",
            Value::Link(_) => "a link",
            Value::Bool(_) => "a boolean",
            Value::Currency(_) => "a currency amount",
            Value::Html(_) => "HTML",
            Value::Markdown(_) => "Markdown",
            Value::List(_) => "a list",
            Value::Record(_) => "a record",
        }
    }
}

/// Markup that is written as it is. Only [`Html::trusted`] makes one; the
/// crate never turns another value into `Html`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Html(String);

impl Html {
    /// Markup the caller vouches for.
    pub fn trusted(markup: impl Into<String>) -> Self {
        Self(markup.into())
    }

    /// The markup.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The markup, owned.
    pub fn into_string(self) -> String {
        self.0
    }
}

/// A calendar day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// The year.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, from 1.
    pub day: u8,
}

impl Date {
    /// `YYYY-MM-DD`, exactly: four digits, `-`, two, `-`, two, naming a day
    /// that exists (leap years by the Gregorian rule).
    pub fn parse(text: &str) -> Option<Date> {
        let b = text.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let digits = |r: std::ops::Range<usize>| {
            b[r.clone()]
                .iter()
                .all(u8::is_ascii_digit)
                .then(|| text[r].parse::<u32>().ok())
                .flatten()
        };
        let (year, month, day) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            1..=12 => 31,
            _ => return None,
        };
        (1..=days).contains(&day).then_some(Date {
            year: year as i32,
            month: month as u8,
            day: day as u8,
        })
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
