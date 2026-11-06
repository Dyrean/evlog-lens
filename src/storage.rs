use crate::{query::SavedView, workspace::LogBatch};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_SAVE: AtomicU64 = AtomicU64::new(0);

pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn discover() -> Self {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
            });
        Self::from_data_home(&base)
    }

    fn from_data_home(base: &Path) -> Self {
        let current = base.join("evlog-lens");
        let legacy = base.join("evlog-explorer-rust");
        // Existing installations must keep their saved sessions and views after the rename.
        Self::new(if !current.exists() && legacy.exists() {
            legacy
        } else {
            current
        })
    }

    pub fn load_views(&self) -> Result<Vec<SavedView>, String> {
        let path = self.root.join("views.json");
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(format!(
                    "Could not read saved views {}: {error}",
                    path.display()
                ));
            }
        };
        let views: Vec<SavedView> = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Could not decode saved views {}: {error}", path.display()))?;
        validate_views(&views)?;
        Ok(views)
    }

    pub fn save_views(&self, views: &[SavedView]) -> Result<(), String> {
        validate_views(views)?;
        let bytes = serde_json::to_vec(views)
            .map_err(|error| format!("Could not encode saved views: {error}"))?;
        write_private(&self.root.join("views.json"), &bytes)
    }

    pub fn load_session(&self) -> Result<LogBatch, String> {
        let path = self.root.join("session.json");
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("Could not read saved session {}: {error}", path.display()))?;
        let batch: LogBatch = serde_json::from_slice(&bytes).map_err(|error| {
            format!("Could not decode saved session {}: {error}", path.display())
        })?;
        batch
            .validate()
            .map_err(|error| format!("Saved session is invalid: {error}"))?;
        Ok(batch)
    }

    pub fn save_session(&self, batch: &LogBatch) -> Result<(), String> {
        batch
            .validate()
            .map_err(|error| format!("Cannot save invalid session: {error}"))?;
        let bytes = serde_json::to_vec(batch)
            .map_err(|error| format!("Could not encode saved session: {error}"))?;
        write_private(&self.root.join("session.json"), &bytes)
    }

    pub fn delete_session(&self) -> Result<(), String> {
        let path = self.root.join("session.json");
        std::fs::remove_file(&path)
            .map_err(|error| format!("Could not delete saved session {}: {error}", path.display()))
    }

    pub fn has_session(&self) -> bool {
        self.root.join("session.json").is_file()
    }
}

fn validate_views(views: &[SavedView]) -> Result<(), String> {
    for view in views {
        if view.name.trim().is_empty() {
            return Err("A saved view has an empty name.".into());
        }
        view.filters
            .validate_persisted()
            .map_err(|error| format!("Saved view {:?} is invalid: {error}", view.name))?;
    }
    Ok(())
}

struct PendingSave(PathBuf);

impl Drop for PendingSave {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Storage path {} has no parent", path.display()))?;
    std::fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Could not create storage directory {}: {error}",
            parent.display()
        )
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("Storage path {} has no filename", path.display()))?;
    let (pending, mut file) = loop {
        let serial = NEXT_SAVE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = std::ffi::OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".{}.{serial}.tmp", std::process::id()));
        let temporary = parent.join(temporary_name);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temporary) {
            Ok(file) => break (PendingSave(temporary), file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Could not create a temporary save in {}: {error}",
                    parent.display()
                ));
            }
        }
    };
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    written.map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    std::fs::rename(&pending.0, path)
        .map_err(|error| format!("Could not publish saved data {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{importer, query::Query, test_support::TempDir};

    fn view(name: &str) -> SavedView {
        SavedView {
            name: name.into(),
            filters: Query::default(),
        }
    }

    #[test]
    fn views_round_trip_and_report_corruption_without_overwriting_it() {
        let directory = TempDir::new();
        let store = Store::new(directory.path().into());
        assert!(store.load_views().unwrap().is_empty());
        store.save_views(&[view("Errors")]).unwrap();
        assert_eq!(store.load_views().unwrap()[0].name, "Errors");
        let path = directory.write("views.json", "corrupt JSON");
        assert!(
            store
                .load_views()
                .err()
                .unwrap()
                .contains("decode saved views")
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), "corrupt JSON");
    }

    #[test]
    fn rejects_invalid_persisted_filters_and_session_metadata() {
        let directory = TempDir::new();
        let store = Store::new(directory.path().into());
        directory.write(
            "views.json",
            r#"[{"name":"Broken","filters":{"minutes":9223372036854775807}}]"#,
        );
        assert!(store.load_views().is_err());
        directory.write("session.json", r#"{"events":[{"raw":{"message":"event"},"file":"missing.json","source":3,"line":1}],"errors":[],"files":[]}"#);
        assert!(store.load_session().is_err());
    }

    #[test]
    fn saved_sessions_preserve_events_and_delete_only_the_saved_copy() {
        let directory = TempDir::new();
        let input = directory.write("input.ndjson", "{\"message\":\"event\"}\n");
        let batch = importer::read_files(&[input], &|| false, |_| {}).unwrap();
        let store = Store::new(directory.path().join("saved"));
        assert!(!store.has_session());
        store.save_session(&batch).unwrap();
        assert!(store.has_session());
        let loaded = store.load_session().unwrap();
        assert_eq!(
            serde_json::to_value(loaded).unwrap(),
            serde_json::to_value(&batch).unwrap()
        );
        store.delete_session().unwrap();
        assert!(!store.has_session());
        assert_eq!(batch.events.len(), 1);
    }

    #[test]
    fn discovers_legacy_data_only_when_the_new_location_is_absent() {
        let directory = TempDir::new();
        let legacy = directory.path().join("evlog-explorer-rust");
        std::fs::create_dir(&legacy).unwrap();
        Store::new(legacy.clone())
            .save_views(&[view("Legacy")])
            .unwrap();
        assert_eq!(
            Store::from_data_home(directory.path())
                .load_views()
                .unwrap()[0]
                .name,
            "Legacy"
        );
        let current = directory.path().join("evlog-lens");
        std::fs::create_dir(&current).unwrap();
        Store::new(current).save_views(&[view("Current")]).unwrap();
        assert_eq!(
            Store::from_data_home(directory.path())
                .load_views()
                .unwrap()[0]
                .name,
            "Current"
        );
    }

    #[test]
    fn concurrent_saves_leave_complete_json_and_no_temporary_files() {
        let directory = TempDir::new();
        std::thread::scope(|scope| {
            for i in 0..8 {
                let root = directory.path();
                scope.spawn(move || {
                    Store::new(root.into())
                        .save_views(&[view(&format!("View {i}"))])
                        .unwrap();
                });
            }
        });
        assert_eq!(
            Store::new(directory.path().into())
                .load_views()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(directory.path().join("views.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn failed_save_cleans_up_the_temporary_file() {
        let directory = TempDir::new();
        std::fs::create_dir(directory.path().join("views.json")).unwrap();
        assert!(
            Store::new(directory.path().into())
                .save_views(&[view("Errors")])
                .is_err()
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
