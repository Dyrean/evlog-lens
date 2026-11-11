use crate::{log, workspace::LogBatch};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    thread::JoinHandle,
};

pub enum ImportUpdate {
    Progress(String),
    Complete(LogBatch),
    Failed(String),
}

pub struct ImportJob {
    receiver: Receiver<ImportUpdate>,
    cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    terminal: bool,
}

impl ImportJob {
    pub fn start(paths: Vec<PathBuf>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let stopped = cancelled.clone();
        let worker = std::thread::spawn(move || {
            let batch = read_files(&paths, &|| stopped.load(Ordering::Relaxed), |path| {
                if sender
                    .send(ImportUpdate::Progress(path.display().to_string()))
                    .is_err()
                {
                    stopped.store(true, Ordering::Relaxed);
                }
            });
            if let Some(batch) = batch {
                let _ = sender.send(ImportUpdate::Complete(batch));
            }
        });
        Self {
            receiver,
            cancelled,
            worker: Some(worker),
            terminal: false,
        }
    }

    pub fn next(&mut self) -> Option<ImportUpdate> {
        if self.terminal {
            return None;
        }
        let update = match self.receiver.try_recv() {
            Ok(update) => update,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => ImportUpdate::Failed(
                "The import worker stopped before completing its batch.".into(),
            ),
        };
        self.terminal = matches!(update, ImportUpdate::Complete(_) | ImportUpdate::Failed(_));
        Some(update)
    }

    pub fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.terminal = true;
    }

    #[cfg(test)]
    pub fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

impl Drop for ImportJob {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take()
            && worker.is_finished()
        {
            let _ = worker.join();
        }
    }
}

fn read_text(path: &Path, cancelled: &dyn Fn() -> bool) -> std::io::Result<Option<String>> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 65_536];
    loop {
        if cancelled() {
            return Ok(None);
        }
        let count = file.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

pub fn read_files(
    paths: &[PathBuf],
    cancelled: &dyn Fn() -> bool,
    mut progress: impl FnMut(&Path),
) -> Option<LogBatch> {
    let mut batch = LogBatch::default();
    for path in paths {
        if cancelled() {
            return None;
        }
        progress(path);
        if cancelled() {
            return None;
        }
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let source = batch.files.len();
        batch.files.push(file_name.clone());
        let input = match read_text(path, cancelled) {
            Ok(Some(input)) => input,
            Ok(None) => return None,
            Err(error) => {
                batch.errors.push(log::ImportError {
                    file: file_name,
                    source: Some(source),
                    line: None,
                    message: format!("Could not read {}: {error}", path.display()),
                });
                continue;
            }
        };
        let mut parsed = log::parse_cancellable(&file_name, source, &input, cancelled)?;
        batch.events.append(&mut parsed.events);
        batch.errors.append(&mut parsed.errors);
    }
    if cancelled() { None } else { Some(batch) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::time::{Duration, Instant};

    fn complete(job: &mut ImportJob) -> Result<LogBatch, String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match job.next() {
                Some(ImportUpdate::Complete(batch)) => return Ok(batch),
                Some(ImportUpdate::Failed(error)) => return Err(error),
                _ => {}
            }
            assert!(Instant::now() < deadline, "Import did not finish");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn unreadable_files_and_bad_lines_do_not_discard_valid_events() {
        let directory = TempDir::new();
        let first = directory.write("first.ndjson", "{\"message\":\"first\"}\nbroken\n");
        let second = directory.write("second.json", "{\"message\":\"second\"}");
        let missing = directory.path().join("missing.json");
        let paths = vec![first, missing, second];
        let mut progress = Vec::new();
        let batch = read_files(&paths, &|| false, |path| progress.push(path.to_owned())).unwrap();
        assert_eq!(progress, paths);
        assert_eq!(batch.events.len(), 2);
        assert_eq!(batch.errors.len(), 2);
        assert_eq!(batch.files, ["first.ndjson", "missing.json", "second.json"]);
        batch.validate().unwrap();
    }

    #[test]
    fn background_and_synchronous_imports_produce_the_same_batch() {
        let directory = TempDir::new();
        let file = directory.write(
            "events.ndjson",
            include_str!("../examples/sample-evlog.ndjson"),
        );
        let expected = read_files(std::slice::from_ref(&file), &|| false, |_| {}).unwrap();
        let mut job = ImportJob::start(vec![file]);
        let actual = complete(&mut job).unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        assert!(job.next().is_none());
    }

    #[test]
    fn cancellation_stops_the_worker_without_publishing_a_partial_batch() {
        let directory = TempDir::new();
        let file = directory.write("large.ndjson", "{\"message\":\"event\"}\n".repeat(100_000));
        let mut job = ImportJob::start(vec![file]);
        job.cancel();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !job.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(job.is_finished(), "Cancelled worker did not exit");
        assert!(
            job.next().is_none(),
            "Cancelled imports must not publish a batch"
        );
    }
}
