use crate::log::{self, Event, Term};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const SLOW_THRESHOLD_MS: f64 = 1000.0;

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filters {
    pub level: String,
    pub file: Option<usize>,
    pub method: String,
    pub status: u16,
    pub slow: bool,
    pub minutes: i64,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Query {
    #[serde(flatten)]
    pub filters: Filters,
    pub query: String,
    pub start: String,
    pub end: String,
    #[serde(skip)]
    pub ascending: bool,
    #[serde(skip)]
    pub correlation: Option<Correlation>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedView {
    pub name: String,
    pub filters: Query,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrelationField {
    Request,
    Trace,
    Session,
    User,
}
impl CorrelationField {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Trace => "trace",
            Self::Session => "session",
            Self::User => "user",
        }
    }
}
impl fmt::Display for CorrelationField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correlation {
    pub field: CorrelationField,
    pub value: String,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum InspectorTab {
    #[default]
    Fields,
    Json,
    Timeline,
    Session,
}
impl InspectorTab {
    pub const ALL: [Self; 4] = [Self::Fields, Self::Json, Self::Timeline, Self::Session];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fields => "Fields",
            Self::Json => "JSON",
            Self::Timeline => "Timeline",
            Self::Session => "Session",
        }
    }
    pub const fn key(self) -> &'static str {
        match self {
            Self::Fields => "tab-fields",
            Self::Json => "tab-json",
            Self::Timeline => "tab-timeline",
            Self::Session => "tab-session",
        }
    }
}
#[derive(Clone, Copy)]
pub enum TimelineKind {
    Related,
    Session,
}

impl Query {
    pub fn saved(&self) -> Self {
        let mut query = self.clone();
        query.filters.file = None;
        query.correlation = None;
        query.ascending = false;
        query
    }
    pub fn is_active(&self) -> bool {
        self.filters != Filters::default()
            || !self.query.is_empty()
            || !self.start.is_empty()
            || !self.end.is_empty()
            || self.correlation.is_some()
    }
    pub fn validate_persisted(&self) -> Result<(), String> {
        if ![
            "", "trace", "debug", "info", "warn", "error", "fatal", "unknown", "errors",
        ]
        .contains(&self.filters.level.as_str())
        {
            return Err("Unrecognized severity filter".into());
        }
        if ![0, 2, 3, 4, 5].contains(&self.filters.status)
            && !(100..=599).contains(&self.filters.status)
        {
            return Err("Unrecognized HTTP status filter".into());
        }
        if ![0, 15, 60, 1440].contains(&self.filters.minutes) {
            return Err("Unrecognized relative time range".into());
        }
        Ok(())
    }
    pub fn compile(&self, latest: Option<i64>) -> Result<CompiledQuery<'_>, String> {
        self.validate_persisted()?;
        let terms = log::search_terms(&self.query)?;
        let bound = |value: &str| -> Result<Option<i64>, String> {
            if value.trim().is_empty() {
                return Ok(None);
            }
            chrono::DateTime::parse_from_rfc3339(value.trim())
                .map(|t| Some(t.timestamp_millis()))
                .map_err(|_| "Use ISO 8601 dates with timezone, e.g. 2026-11-06T09:00:00Z".into())
        };
        let mut start = bound(&self.start)?;
        let end = bound(&self.end)?;
        if start.zip(end).is_some_and(|(a, b)| a > b) {
            return Err("Start must be before end.".into());
        }
        if self.filters.minutes != 0 {
            let relative = latest.map(|t| t.saturating_sub(self.filters.minutes * 60_000));
            if let Some(relative) = relative {
                start = Some(start.map_or(relative, |s| s.max(relative)));
            }
        }
        Ok(CompiledQuery {
            query: self,
            terms,
            start,
            end,
            has_relative_anchor: self.filters.minutes == 0 || latest.is_some(),
        })
    }
}

pub struct CompiledQuery<'a> {
    query: &'a Query,
    terms: Vec<Term>,
    start: Option<i64>,
    end: Option<i64>,
    has_relative_anchor: bool,
}
impl CompiledQuery<'_> {
    pub fn matches(&self, event: &Event) -> bool {
        let f = &self.query.filters;
        let p = event.projection();
        log::matches(event, &self.terms)
            && (f.level.is_empty()
                || if f.level == "errors" {
                    event.is_error()
                } else {
                    event.level() == f.level
                })
            && f.file.is_none_or(|id| event.source() == id)
            && (f.method.is_empty() || p.method.eq_ignore_ascii_case(&f.method))
            && (f.status == 0
                || event.status().is_some_and(|n| {
                    if f.status < 100 {
                        n / 100 == f.status
                    } else {
                        n == f.status
                    }
                }))
            && (!f.slow
                || (!p.method.is_empty() && p.duration.is_some_and(|n| n >= SLOW_THRESHOLD_MS)))
            && self.has_relative_anchor
            && self
                .start
                .is_none_or(|s| p.timestamp.is_some_and(|t| t >= s))
            && self.end.is_none_or(|e| p.timestamp.is_some_and(|t| t <= e))
            && self
                .query
                .correlation
                .as_ref()
                .is_none_or(|c| event.field(c.field.key()) == c.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_views_keep_filters_but_exclude_source_and_followed_ids() {
        let query = Query {
            filters: Filters {
                file: Some(2),
                minutes: 60,
                ..Default::default()
            },
            query: "status:503".into(),
            correlation: Some(Correlation {
                field: CorrelationField::Session,
                value: "s".into(),
            }),
            ..Default::default()
        };
        let saved = query.saved();
        assert_eq!(saved.query, "status:503");
        assert_eq!(saved.filters.minutes, 60);
        assert!(saved.filters.file.is_none());
        assert!(saved.correlation.is_none());
        let legacy = r#"{"name":"Errors","filters":{"query":"status:503","level":"errors","file":null,"method":"","status":0,"slow":false,"minutes":0,"start":"","end":""}}"#;
        let view: SavedView = serde_json::from_str(legacy).unwrap();
        assert_eq!(view.filters.query, "status:503");
        assert_eq!(view.filters.filters.level, "errors");
    }
    #[test]
    fn exact_status_and_status_groups_are_distinct_and_can_be_saved() {
        let events = log::parse(
            "test.json",
            0,
            r#"[{"status":503},{"status":500},{"status":200},{}]"#,
        )
        .events;
        for (status, count) in [(0, 4), (5, 2), (503, 1), (200, 1)] {
            let query = Query {
                filters: Filters {
                    status,
                    ..Default::default()
                },
                ..Default::default()
            };
            let query: Query =
                serde_json::from_str(&serde_json::to_string(&query).unwrap()).unwrap();
            let compiled = query.compile(None).unwrap();
            assert_eq!(events.iter().filter(|e| compiled.matches(e)).count(), count);
        }
        for status in [1, 6, 99, 600, u16::MAX] {
            let query = Query {
                filters: Filters {
                    status,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert!(query.compile(None).is_err());
        }
    }
    #[test]
    fn persisted_time_ranges_cannot_overflow_filter_arithmetic() {
        let query = Query {
            filters: Filters {
                minutes: i64::MAX,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(query.compile(Some(0)).is_err());
    }
}
