//! Immutable source events and normalized projections.
use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    raw: Value,
    file: String,
    source: usize,
    line: Option<usize>,
    #[serde(skip)]
    cached: OnceLock<Projection>,
}
#[derive(Clone, Debug)]
pub struct Projection {
    pub timestamp: Option<i64>,
    pub level: String,
    pub status: Option<u16>,
    pub duration: Option<f64>,
    pub search: String,
    pub request: String,
    pub trace: String,
    pub session: String,
    pub method: String,
}
pub fn at<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(value, |v, key| v.get(key))
}
pub fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(_) | Value::Bool(_) => Some(v.to_string()),
        _ => None,
    }
}
fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str()?.trim().parse().ok())
        .filter(|n| n.is_finite())
}
impl Event {
    pub(super) fn new(raw: Value, file: &str, source: usize, line: Option<usize>) -> Self {
        let event = Self {
            raw,
            file: file.into(),
            source,
            line,
            cached: OnceLock::new(),
        };
        event.projection();
        event
    }
    pub fn projection(&self) -> &Projection {
        self.cached.get_or_init(|| Projection::from_raw(&self.raw))
    }
    pub fn raw(&self) -> &Value {
        &self.raw
    }
    pub fn file(&self) -> &str {
        &self.file
    }
    pub fn source(&self) -> usize {
        self.source
    }
    pub fn line(&self) -> Option<usize> {
        self.line
    }
    pub(crate) fn set_source(&mut self, source: usize) {
        self.source = source;
    }
    pub fn level(&self) -> &str {
        &self.projection().level
    }
    pub fn status(&self) -> Option<u16> {
        self.projection().status
    }
    pub fn duration(&self) -> Option<f64> {
        self.projection().duration
    }
    pub fn timestamp(&self) -> Option<i64> {
        self.projection().timestamp
    }
    pub fn value(&self, field: &str) -> Option<&Value> {
        value_at(&self.raw, field)
    }
    pub fn field(&self, key: &str) -> String {
        match key {
            "request" | "requestId" => self.projection().request.clone(),
            "trace" | "traceId" => self.projection().trace.clone(),
            "session" | "sessionId" | "sid" => self.projection().session.clone(),
            "method" => self.projection().method.clone(),
            _ => self.read_field(key),
        }
    }
    fn read_field(&self, key: &str) -> String {
        self.value(key).and_then(scalar).unwrap_or_default()
    }
    pub fn time(&self, utc: bool) -> String {
        self.timestamp()
            .and_then(|ts| Utc.timestamp_millis_opt(ts).single())
            .map(|t| {
                if utc {
                    t.format("%H:%M:%S%.3f").to_string()
                } else {
                    t.with_timezone(&Local).format("%H:%M:%S%.3f").to_string()
                }
            })
            .unwrap_or_else(|| "—".into())
    }
    pub fn is_error(&self) -> bool {
        matches!(self.level(), "error" | "fatal")
    }
}
pub fn duration_label(n: Option<f64>) -> String {
    match n {
        Some(n) if n >= 1000.0 => format!("{:.2} s", n / 1000.0),
        Some(n) => format!("{n:.0} ms"),
        None => "—".into(),
    }
}

const STATUS_PATHS: &[&str] = &[
    "status",
    "statusCode",
    "response.status",
    "response.statusCode",
    "http.status_code",
    "http.response.status_code",
];
fn read_scalar(raw: &Value, field: &str) -> String {
    value_at(raw, field).and_then(scalar).unwrap_or_default()
}
impl Projection {
    fn from_raw(raw: &Value) -> Self {
        Self {
            timestamp: read_timestamp(raw),
            level: read_level(raw),
            status: read_status(raw),
            duration: read_duration(raw),
            search: raw.to_string().to_lowercase(),
            request: read_scalar(raw, "request"),
            trace: read_scalar(raw, "trace"),
            session: read_scalar(raw, "session"),
            method: read_scalar(raw, "method"),
        }
    }
}
fn value_at<'a>(raw: &'a Value, field: &str) -> Option<&'a Value> {
    let paths: &[&str] = match field {
        "timestamp" => &[
            "timestamp",
            "time",
            "ts",
            "@timestamp",
            "datetime",
            "createdAt",
        ],
        "level" => &["level", "severity", "log.level", "severityText"],
        "message" => &["message", "msg", "event.message", "error.message"],
        "method" => &[
            "method",
            "request.method",
            "http.method",
            "http.request.method",
        ],
        "path" => &[
            "path",
            "request.path",
            "request.url",
            "url",
            "route",
            "http.path",
            "http.route",
            "event",
        ],
        "status" => STATUS_PATHS,
        "duration" => &[
            "duration",
            "durationMs",
            "duration_ms",
            "elapsed",
            "elapsedMs",
            "latency",
            "response.duration",
            "request.duration",
        ],
        "request" | "requestId" => &[
            "requestId",
            "request_id",
            "request.id",
            "context.requestId",
            "correlationId",
        ],
        "trace" | "traceId" => &["traceId", "trace_id", "trace.id", "context.traceId"],
        "session" | "sessionId" | "sid" => &[
            "session.id",
            "sessionId",
            "session_id",
            "context.sessionId",
            "context.session.id",
            "sid",
        ],
        "user" | "userId" => &["userId", "user_id", "user.id", "context.userId"],
        "why" => &["why", "error.why"],
        "fix" => &["fix", "error.fix"],
        _ => return at(raw, field),
    };
    paths.iter().filter_map(|p| at(raw, p)).find(|v| {
        if matches!(field, "session" | "sessionId" | "sid") {
            v.as_str().is_some_and(|id| !id.trim().is_empty())
        } else {
            !v.is_null() && v.as_str() != Some("")
        }
    })
}
fn read_level(raw: &Value) -> String {
    match read_scalar(raw, "level").trim().to_lowercase().as_str() {
        "warning" => "warn",
        "critical" | "panic" => "fatal",
        "trace" => "trace",
        "debug" => "debug",
        "info" => "info",
        "warn" => "warn",
        "error" => "error",
        "fatal" => "fatal",
        _ => "unknown",
    }
    .into()
}
fn read_status(raw: &Value) -> Option<u16> {
    STATUS_PATHS
        .iter()
        .filter_map(|p| at(raw, p).and_then(number))
        .find(|n| (100.0..=599.0).contains(n) && n.fract() == 0.0)
        .map(|n| n as u16)
}
fn read_duration(raw: &Value) -> Option<f64> {
    let v = value_at(raw, "duration")?;
    let n = number(v).or_else(|| {
        let s = v.as_str()?.trim().to_lowercase();
        for (suffix, multiplier) in [("ms", 1.0), ("µs", 0.001), ("us", 0.001), ("s", 1000.0)] {
            if let Some(n) = s.strip_suffix(suffix) {
                return n.trim().parse::<f64>().ok().map(|n| n * multiplier);
            }
        }
        None
    })?;
    (n.is_finite() && n >= 0.0).then_some(n)
}
fn read_timestamp(raw: &Value) -> Option<i64> {
    let v = value_at(raw, "timestamp")?;
    if let Some(n) = number(v) {
        let ms = if n.abs() < 10_000_000_000.0 {
            n * 1000.0
        } else {
            n
        };
        return DateTime::from_timestamp_millis(ms as i64).map(|t| t.timestamp_millis());
    }
    DateTime::parse_from_rfc3339(v.as_str()?)
        .ok()
        .map(|t| t.timestamp_millis())
}
