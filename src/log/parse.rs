use super::Event;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportError {
    pub file: String,
    #[serde(default)]
    pub source: Option<usize>,
    pub line: Option<usize>,
    pub message: String,
}
#[derive(Default)]
pub struct Batch {
    pub events: Vec<Event>,
    pub errors: Vec<ImportError>,
}

pub fn parse(file: &str, source: usize, input: &str) -> Batch {
    parse_cancellable(file, source, input, &|| false).unwrap_or_default()
}

fn decode(input: &str, cancelled: &dyn Fn() -> bool) -> serde_json::Result<Value> {
    if input.len() < 65536 {
        return serde_json::from_str(input);
    }
    struct CheckedReader<'a> {
        bytes: &'a [u8],
        cancelled: &'a dyn Fn() -> bool,
        read: usize,
    }
    impl std::io::Read for CheckedReader<'_> {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            if self.read.is_multiple_of(4096) && (self.cancelled)() {
                return Err(std::io::Error::other("Import cancelled"));
            }
            let n = std::io::Read::read(&mut self.bytes, out)?;
            self.read += n;
            Ok(n)
        }
    }
    serde_json::from_reader(CheckedReader {
        bytes: input.as_bytes(),
        cancelled,
        read: 0,
    })
}

pub fn parse_cancellable(
    file: &str,
    source: usize,
    input: &str,
    cancelled: &dyn Fn() -> bool,
) -> Option<Batch> {
    if cancelled() {
        return None;
    }
    let input = input.trim_start_matches('\u{feff}');
    let mut batch = Batch::default();
    let mut add = |v: Value, line| {
        if v.is_object() {
            batch.events.push(Event::new(v, file, source, line));
        } else {
            batch.errors.push(ImportError {
                file: file.into(),
                source: Some(source),
                line,
                message: "Event must be a JSON object".into(),
            });
        }
    };
    let line_file =
        file.to_lowercase().ends_with(".ndjson") || file.to_lowercase().ends_with(".jsonl");
    if !line_file {
        let decoded = decode(input, cancelled);
        if cancelled() {
            return None;
        }
        match decoded {
            Ok(Value::Array(items)) => {
                for v in items {
                    if cancelled() {
                        return None;
                    }
                    add(v, None);
                }
                return Some(batch);
            }
            Ok(mut v) => {
                if let Some(Value::Array(items)) = v.get_mut("events") {
                    for item in std::mem::take(items) {
                        if cancelled() {
                            return None;
                        }
                        add(item, None);
                    }
                } else {
                    add(v, None);
                }
                return Some(batch);
            }
            Err(_) if input.trim_start().starts_with('[') => {
                batch.errors.push(ImportError {
                    file: file.into(),
                    source: Some(source),
                    line: None,
                    message: "Invalid JSON array".into(),
                });
                return Some(batch);
            }
            Err(_) => {}
        }
    }
    for (i, line) in input.lines().enumerate() {
        if cancelled() {
            return None;
        }
        if line.trim().is_empty() {
            continue;
        }
        match decode(line, cancelled) {
            Ok(v) if v.is_object() => batch.events.push(Event::new(v, file, source, Some(i + 1))),
            Ok(_) => batch.errors.push(ImportError {
                file: file.into(),
                source: Some(source),
                line: Some(i + 1),
                message: "Event must be a JSON object".into(),
            }),
            Err(e) => batch.errors.push(ImportError {
                file: file.into(),
                source: Some(source),
                line: Some(i + 1),
                message: e.to_string(),
            }),
        }
    }
    if cancelled() { None } else { Some(batch) }
}
