use crate::{
    log::{Event, ImportError},
    query::{Query, SLOW_THRESHOLD_MS, TimelineKind},
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[derive(Default, Serialize, Deserialize)]
pub struct LogBatch {
    pub events: Vec<Event>,
    pub errors: Vec<ImportError>,
    pub files: Vec<String>,
}
impl LogBatch {
    pub fn validate(&self) -> Result<(), String> {
        for event in &self.events {
            if !event.raw().is_object() {
                return Err("Saved events must be JSON objects".into());
            }
            if self.files.get(event.source()).map(String::as_str) != Some(event.file()) {
                return Err("Saved event source metadata does not match its file".into());
            }
        }
        Ok(())
    }
    pub fn sample() -> Self {
        let name = "sample-evlog.ndjson";
        let parsed = crate::log::parse(name, 0, include_str!("../examples/sample-evlog.ndjson"));
        Self {
            events: parsed.events,
            errors: parsed.errors,
            files: vec![name.into()],
        }
    }
}
#[derive(Clone, Default)]
pub struct Stats {
    pub errors: usize,
    pub warnings: usize,
    pub requests: usize,
    pub slow: usize,
    pub p95: Option<f64>,
}
pub struct QueryResult {
    pub ids: Vec<usize>,
    pub stats: Stats,
}
struct CachedQuery {
    query: Query,
    result: Result<Rc<QueryResult>, String>,
}
#[derive(Default)]
pub struct Workspace {
    generation: u64,
    data: LogBatch,
    query: RefCell<Option<CachedQuery>>,
    totals: RefCell<Option<Stats>>,
}
pub struct TimelineGroup {
    pub label: Option<String>,
    pub events: Vec<usize>,
}
impl Workspace {
    pub fn events(&self) -> &[Event] {
        &self.data.events
    }
    pub fn files(&self) -> &[String] {
        &self.data.files
    }
    pub fn errors(&self) -> &[ImportError] {
        &self.data.errors
    }
    pub fn snapshot(&self) -> &LogBatch {
        &self.data
    }
    pub fn append(&mut self, mut batch: LogBatch) -> Result<(), String> {
        batch.validate()?;
        for event in &mut batch.events {
            event.rebase_source(self.data.files.len());
            event.projection();
        }
        self.data.events.append(&mut batch.events);
        self.data.errors.append(&mut batch.errors);
        self.data.files.append(&mut batch.files);
        self.invalidate();
        Ok(())
    }
    pub fn replace(&mut self, batch: LogBatch) -> Result<(), String> {
        batch.validate()?;
        for event in &batch.events {
            event.projection();
        }
        self.data = batch;
        self.invalidate();
        Ok(())
    }
    pub fn clear(&mut self) {
        self.data = LogBatch::default();
        self.invalidate();
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.query.get_mut().take();
        self.totals.get_mut().take();
    }
    pub fn results(&self, query: &Query) -> Result<Rc<QueryResult>, String> {
        if let Some(cached) = self.query.borrow().as_ref().filter(|c| c.query == *query) {
            return cached.result.clone();
        }
        let result = (|| {
            let compiled =
                query.compile(self.events().iter().filter_map(Event::timestamp).max())?;
            let mut ids: Vec<_> = self
                .events()
                .iter()
                .enumerate()
                .filter(|(_, event)| compiled.matches(event))
                .map(|(i, _)| i)
                .collect();
            self.sort(&mut ids, query.ascending);
            let stats = self.summarize(ids.iter().copied());
            Ok(Rc::new(QueryResult { ids, stats }))
        })();
        *self.query.borrow_mut() = Some(CachedQuery {
            query: query.clone(),
            result: result.clone(),
        });
        result
    }
    pub fn totals(&self) -> Stats {
        if let Some(stats) = self.totals.borrow().as_ref() {
            return stats.clone();
        }
        let stats = self.summarize(0..self.events().len());
        *self.totals.borrow_mut() = Some(stats.clone());
        stats
    }
    fn summarize(&self, ids: impl Iterator<Item = usize>) -> Stats {
        let mut stats = Stats::default();
        let mut requests = HashSet::new();
        let mut durations = Vec::new();
        for i in ids {
            let event = &self.events()[i];
            let p = event.projection();
            stats.errors += usize::from(event.is_error());
            stats.warnings += usize::from(p.level == "warn");
            if !p.request.is_empty() {
                requests.insert(p.request.as_str());
            }
            if !p.method.is_empty()
                && let Some(duration) = p.duration
            {
                durations.push(duration);
                stats.slow += usize::from(duration >= SLOW_THRESHOLD_MS);
            }
        }
        stats.requests = requests.len();
        durations.sort_by(f64::total_cmp);
        stats.p95 = durations
            .get(((durations.len() as f64 * 0.95).ceil() as usize).saturating_sub(1))
            .copied();
        stats
    }
    fn sort(&self, ids: &mut [usize], ascending: bool) {
        ids.sort_by(|a, b| {
            chronological(&self.events()[*a], &self.events()[*b], ascending).then(a.cmp(b))
        });
    }
    pub fn timeline(&self, index: usize, kind: TimelineKind) -> Vec<TimelineGroup> {
        let Some(selected) = self.events().get(index) else {
            return vec![];
        };
        let p = selected.projection();
        let mut ids: Vec<_> = self
            .events()
            .iter()
            .enumerate()
            .filter(|(_, event)| {
                let other = event.projection();
                match kind {
                    TimelineKind::Session => !p.session.is_empty() && p.session == other.session,
                    TimelineKind::Related => {
                        (!p.request.is_empty() && p.request == other.request)
                            || (!p.trace.is_empty() && p.trace == other.trace)
                    }
                }
            })
            .map(|(i, _)| i)
            .collect();
        self.sort(&mut ids, true);
        if matches!(kind, TimelineKind::Related) {
            return ids
                .into_iter()
                .map(|i| TimelineGroup {
                    label: None,
                    events: vec![i],
                })
                .collect();
        }
        let mut groups: Vec<TimelineGroup> = Vec::new();
        let mut requests = HashMap::new();
        for i in ids {
            let request = &self.events()[i].projection().request;
            if !request.is_empty()
                && let Some(group) = requests.get(request.as_str()).copied()
            {
                let group: usize = group;
                groups[group].events.push(i);
                continue;
            }
            if !request.is_empty() {
                requests.insert(request.as_str(), groups.len());
            }
            groups.push(TimelineGroup {
                label: (!request.is_empty()).then(|| request.clone()),
                events: vec![i],
            });
        }
        groups
    }
}
fn chronological(a: &Event, b: &Event, ascending: bool) -> Ordering {
    match (a.timestamp(), b.timestamp()) {
        (Some(a), Some(b)) => {
            if ascending {
                a.cmp(&b)
            } else {
                b.cmp(&a)
            }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn batch(input: &str) -> LogBatch {
        let parsed = crate::log::parse("test.ndjson", 0, input);
        LogBatch {
            events: parsed.events,
            errors: parsed.errors,
            files: vec!["test.ndjson".into()],
        }
    }
    #[test]
    fn replacing_the_same_number_of_events_refreshes_matches_and_summaries() {
        let mut workspace = Workspace::default();
        let query = Query {
            query: "status:503".into(),
            ..Default::default()
        };
        workspace
            .replace(batch("{\"status\":503,\"level\":\"error\"}"))
            .unwrap();
        assert_eq!(workspace.results(&query).unwrap().ids.len(), 1);
        assert_eq!(workspace.totals().errors, 1);
        workspace
            .replace(batch("{\"status\":200,\"level\":\"info\"}"))
            .unwrap();
        assert!(workspace.results(&query).unwrap().ids.is_empty());
        assert_eq!(workspace.totals().errors, 0);
        workspace
            .append(batch("{\"status\":503,\"level\":\"error\"}"))
            .unwrap();
        assert_eq!(workspace.results(&query).unwrap().ids.len(), 1);
        assert_eq!(workspace.events()[1].source(), 1);
    }
    #[test]
    fn timelines_are_chronological_independent_of_filters_and_leave_undated_events_last() {
        let mut workspace = Workspace::default();
        workspace
            .replace(batch(concat!(
                "{\"sessionId\":\"s\",\"requestId\":\"a\",\"timestamp\":3}\n",
                "{\"sessionId\":\"s\",\"requestId\":\"b\",\"timestamp\":2}\n",
                "{\"sessionId\":\"s\",\"requestId\":\"a\",\"timestamp\":1}\n",
                "{\"sessionId\":\"s\"}\n{\"sessionId\":\"s\"}\n"
            )))
            .unwrap();
        let groups = workspace.timeline(0, TimelineKind::Session);
        assert_eq!(groups.len(), 4);
        assert_eq!(groups[0].label.as_deref(), Some("a"));
        assert_eq!(groups[0].events, vec![2, 0]);
        assert_eq!(groups[1].events, vec![1]);
        assert_eq!(groups[2].events, vec![3]);
        assert_eq!(groups[3].events, vec![4]);
        let related = workspace.timeline(0, TimelineKind::Related);
        assert_eq!(
            related
                .iter()
                .flat_map(|g| g.events.clone())
                .collect::<Vec<_>>(),
            vec![2, 0]
        );
        for ascending in [true, false] {
            let result = workspace
                .results(&Query {
                    ascending,
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(&result.ids[3..], &[3, 4]);
        }
    }
    #[test]
    fn invalid_source_metadata_never_replaces_valid_data() {
        let mut workspace = Workspace::default();
        workspace.replace(LogBatch::sample()).unwrap();
        let mut invalid = batch("{}");
        invalid.files.clear();
        assert!(workspace.replace(invalid).is_err());
        assert_eq!(workspace.events().len(), 50);
    }
}
