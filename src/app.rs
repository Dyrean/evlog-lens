use crate::{
    importer::{self, ImportJob, ImportUpdate},
    query::{Correlation, CorrelationField, Filters, InspectorTab, Query, SavedView},
    storage::Store,
    workspace::{LogBatch, QueryResult, Workspace},
};
use rune::{TextEdit, Tx};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    io::Write,
    path::PathBuf,
    rc::Rc,
    time::Duration,
};

pub struct State {
    workspace: Workspace,
    pub search: TextEdit,
    pub start: TextEdit,
    pub end: TextEdit,
    pub view_name: TextEdit,
    pub filters: Filters,
    pub selected: Option<usize>,
    pub correlation: Option<Correlation>,
    pub tab: InspectorTab,
    pub ascending: bool,
    pub utc: bool,
    pub advanced: bool,
    pub show_errors: bool,
    pub notice: String,
    pub views: Vec<SavedView>,
    pub collapsed: HashSet<String>,
    pub drop_over: bool,
    store: Option<Store>,
    views_load_error: Option<String>,
    saved_session: bool,
    pending: Option<ImportJob>,
    progress: String,
    page: Cell<usize>,
    page_query: RefCell<Option<(u64, Query)>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            workspace: Workspace::default(),
            search: TextEdit::new(""),
            start: TextEdit::new(""),
            end: TextEdit::new(""),
            view_name: TextEdit::new(""),
            filters: Filters::default(),
            selected: None,
            correlation: None,
            tab: InspectorTab::Fields,
            ascending: false,
            utc: true,
            advanced: false,
            show_errors: false,
            notice: String::new(),
            views: vec![],
            collapsed: HashSet::new(),
            drop_over: false,
            store: None,
            views_load_error: None,
            saved_session: false,
            pending: None,
            progress: String::new(),
            page: Cell::new(0),
            page_query: RefCell::new(None),
        }
    }
}
impl State {
    pub fn new(store: Store) -> Self {
        let mut state = Self {
            saved_session: store.has_session(),
            ..Default::default()
        };
        match store.load_views() {
            Ok(views) => state.views = views,
            Err(error) => {
                state.notice = error.clone();
                state.views_load_error = Some(error);
            }
        }
        state.store = Some(store);
        state
    }
    pub fn query(&self) -> Query {
        Query {
            filters: self.filters.clone(),
            query: self.search.text.clone(),
            start: self.start.text.clone(),
            end: self.end.text.clone(),
            ascending: self.ascending,
            correlation: self.correlation.clone(),
        }
    }
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }
    pub fn results(&self) -> Result<Rc<QueryResult>, String> {
        let query = self.query();
        let identity = (self.workspace.generation(), query.clone());
        if self.page_query.borrow().as_ref() != Some(&identity) {
            self.page.set(0);
            *self.page_query.borrow_mut() = Some(identity);
        }
        self.workspace.results(&query)
    }
    pub fn query_active(&self) -> bool {
        self.query().is_active()
    }
    pub fn table_page(&self) -> usize {
        let _ = self.results();
        self.page.get()
    }
    pub fn set_page(&mut self, page: usize) {
        self.page.set(page);
    }
    pub fn sample(&mut self) {
        self.clear();
        match self.workspace.replace(LogBatch::sample()) {
            Ok(()) => self.notice = "Sample incident loaded · 12 requests across 4 sessions".into(),
            Err(error) => self.notice = error,
        }
    }
    pub fn clear(&mut self) {
        self.cancel_import();
        self.workspace.clear();
        self.selected = None;
        self.collapsed.clear();
        self.reset();
        self.notice.clear();
    }
    pub fn reset(&mut self) {
        self.filters = Filters::default();
        self.search = TextEdit::new("");
        self.start = TextEdit::new("");
        self.end = TextEdit::new("");
        self.correlation = None;
        self.page.set(0);
    }
    pub fn import(&mut self, paths: Vec<PathBuf>) {
        if self.importing() {
            self.notice =
                "An import is already running. Finish or cancel it before importing more files."
                    .into();
            return;
        }
        self.progress = "Reading local files…".into();
        self.pending = Some(ImportJob::start(paths));
    }
    pub fn load_files(&mut self, paths: &[PathBuf]) {
        if let Some(batch) = importer::read_files(paths, &|| false, |_| {}) {
            self.commit_import(batch);
        }
    }
    fn commit_import(&mut self, batch: LogBatch) {
        let message = format!(
            "Imported {} events · {} issues",
            batch.events.len(),
            batch.errors.len()
        );
        self.notice = match self.workspace.append(batch) {
            Ok(()) => message,
            Err(error) => format!("Could not commit imported logs: {error}"),
        };
    }
    pub fn importing(&self) -> bool {
        self.pending.is_some()
    }
    pub fn import_progress(&self) -> &str {
        &self.progress
    }
    pub fn cancel_import(&mut self) {
        self.pending = None;
    }
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Some(update) = self.pending.as_mut().and_then(ImportJob::next) {
            changed = true;
            match update {
                ImportUpdate::Progress(progress) => self.progress = progress,
                ImportUpdate::Complete(batch) => {
                    self.pending = None;
                    self.commit_import(batch);
                    break;
                }
                ImportUpdate::Failed(error) => {
                    self.pending = None;
                    self.notice = error;
                    break;
                }
            }
        }
        changed
    }
    pub fn follow(&mut self, field: CorrelationField, value: String) {
        self.reset();
        self.correlation = Some(Correlation { field, value });
    }
    fn store(&self) -> Result<&Store, String> {
        self.store
            .as_ref()
            .ok_or_else(|| "No local storage is configured".into())
    }
    pub fn has_saved_session(&self) -> bool {
        self.saved_session
    }
    pub fn save_session(&mut self) {
        match self
            .store()
            .and_then(|store| store.save_session(self.workspace.snapshot()))
        {
            Ok(()) => {
                self.saved_session = true;
                self.notice = "Session saved on this device".into();
            }
            Err(error) => self.notice = error,
        }
    }
    pub fn restore(&mut self) {
        match self.store().and_then(Store::load_session) {
            Ok(batch) => match self.workspace.replace(batch) {
                Ok(()) => {
                    self.cancel_import();
                    self.reset();
                    self.selected = None;
                    self.collapsed.clear();
                    self.notice = "Saved session restored".into();
                }
                Err(error) => self.notice = error,
            },
            Err(error) => self.notice = error,
        }
    }
    pub fn delete_session(&mut self) {
        match self.store().and_then(Store::delete_session) {
            Ok(()) => {
                self.saved_session = false;
                self.notice = "Saved session deleted".into();
            }
            Err(error) => self.notice = error,
        }
    }
    fn editable_views(&mut self) -> Result<Vec<SavedView>, String> {
        if self.views_load_error.is_some() {
            match self.store()?.load_views() {
                Ok(views) => {
                    self.views = views;
                    self.views_load_error = None;
                }
                Err(error) => {
                    self.views_load_error = Some(error.clone());
                    return Err(format!(
                        "Saved views were not loaded: {error}. Repair or remove the saved views file before saving changes."
                    ));
                }
            }
        }
        Ok(self.views.clone())
    }
    pub fn save_view(&mut self) {
        let name = self.view_name.text.trim().to_owned();
        if name.is_empty() {
            self.notice = "Enter a name for this view".into();
            return;
        }
        let result = (|| {
            let mut views = self.editable_views()?;
            views.retain(|v| v.name != name);
            views.push(SavedView {
                name,
                filters: self.query().saved(),
            });
            self.store()?.save_views(&views)?;
            Ok(views)
        })();
        match result {
            Ok(views) => {
                self.views = views;
                self.view_name = TextEdit::new("");
                self.notice = "Views saved on this device".into();
            }
            Err(error) => self.notice = error,
        }
    }
    pub fn delete_view(&mut self, index: usize) {
        let result = (|| {
            let mut views = self.editable_views()?;
            if index >= views.len() {
                return Err("The saved view no longer exists".into());
            }
            views.remove(index);
            self.store()?.save_views(&views)?;
            Ok(views)
        })();
        match result {
            Ok(views) => {
                self.views = views;
                self.notice = "Views saved on this device".into();
            }
            Err(error) => self.notice = error,
        }
    }
    pub fn load_view(&mut self, index: usize) {
        let Some(view) = self.views.get(index) else {
            self.notice = "The saved view no longer exists".into();
            return;
        };
        let query = view.filters.clone();
        self.reset();
        self.filters = query.filters;
        self.search = TextEdit::new(&query.query);
        self.start = TextEdit::new(&query.start);
        self.end = TextEdit::new(&query.end);
    }
    pub fn export(&mut self, path: PathBuf) {
        let result = (|| {
            let result = self.results()?;
            let file = std::fs::File::create(path).map_err(|error| error.to_string())?;
            let mut writer = std::io::BufWriter::new(file);
            for i in &result.ids {
                serde_json::to_writer(&mut writer, self.workspace.events()[*i].raw())
                    .map_err(|error| error.to_string())?;
                writer.write_all(b"\n").map_err(|error| error.to_string())?;
            }
            writer.flush().map_err(|error| error.to_string())
        })();
        self.notice = match result {
            Ok(()) => "Matching events exported as NDJSON".into(),
            Err(error) => format!("Could not export: {error}"),
        };
    }
}
pub fn poll(state: &mut State, _: f64, tx: &mut Tx) -> bool {
    let changed = state.poll();
    if state.importing() {
        tx.wake_after(Duration::from_millis(16));
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    #[test]
    fn saving_views_does_not_overwrite_a_corrupt_store() {
        let dir = TempDir::new();
        let path = dir.write("views.json", "not JSON");
        let mut state = State::new(Store::new(dir.path().into()));
        assert!(state.notice.contains("decode saved views"));
        state.view_name = TextEdit::new("Incident");
        state.save_view();
        assert!(state.notice.contains("Repair or remove"));
        assert!(state.views.is_empty());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "not JSON");
    }
    #[test]
    fn query_edits_reset_pagination_but_inspector_edits_keep_the_current_page() {
        let mut state = State::default();
        state.sample();
        state.results().unwrap();
        state.set_page(3);
        state.selected = Some(0);
        assert_eq!(state.table_page(), 3);
        state.search = TextEdit::new("status:503");
        assert_eq!(state.table_page(), 0);
        state.set_page(2);
        state.start = TextEdit::new("2026-11-06T09:00:00Z");
        assert_eq!(state.table_page(), 0);
    }
    #[test]
    fn sessions_views_and_exports_round_trip_through_the_application() {
        let dir = TempDir::new();
        let mut state = State::new(Store::new(dir.path().into()));
        state.sample();
        state.save_session();
        assert!(state.has_saved_session());
        state.clear();
        state.restore();
        assert_eq!(state.workspace.events().len(), 50);
        state.filters.file = Some(0);
        state.search = TextEdit::new("status:503");
        state.view_name = TextEdit::new("Provider failures");
        state.save_view();
        state.reset();
        state.load_view(0);
        assert!(state.filters.file.is_none());
        assert_eq!(state.results().unwrap().ids.len(), 1);
        let path = dir.path().join("matching.ndjson");
        state.export(path.clone());
        assert_eq!(std::fs::read_to_string(path).unwrap().lines().count(), 1);
        state.delete_session();
        assert!(!state.has_saved_session());
        assert_eq!(state.workspace.events().len(), 50);
    }
    #[test]
    fn importing_a_new_batch_returns_navigation_to_the_first_page() {
        let dir = TempDir::new();
        let mut state = State::default();
        state.sample();
        state.results().unwrap();
        state.set_page(3);
        let file = dir.write("more.ndjson", "{\"message\":\"new event\"}\n");
        state.load_files(&[file]);
        assert_eq!(state.table_page(), 0);
        assert_eq!(state.workspace.events().len(), 51);
    }
    #[test]
    fn repairing_saved_views_recovers_without_losing_the_repaired_views() {
        let dir = TempDir::new();
        dir.write("views.json", "broken");
        let mut state = State::new(Store::new(dir.path().into()));
        Store::new(dir.path().into())
            .save_views(&[SavedView {
                name: "Recovered".into(),
                filters: Query::default(),
            }])
            .unwrap();
        state.view_name = TextEdit::new("New view");
        state.save_view();
        let saved = Store::new(dir.path().into()).load_views().unwrap();
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[0].name, "Recovered");
        assert_eq!(saved[1].name, "New view");
    }
}
