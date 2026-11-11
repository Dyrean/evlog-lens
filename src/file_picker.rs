// shortcut: import dialogs are unparented; use Rune's dialogs once they support multi-selection.
use std::{future::Future, path::PathBuf, pin::Pin};

pub type Selection = Result<Option<Vec<PathBuf>>, String>;
pub type Dialog = Pin<Box<dyn Future<Output = Selection>>>;

#[cfg(all(test, target_os = "linux"))]
mod tests;

#[cfg(target_os = "linux")]
pub fn open() -> Dialog {
    // Ashpd caches its connection; use its independent async backend, not a per-dialog runtime.
    Box::pin(select_files())
}

#[cfg(target_os = "linux")]
async fn select_files() -> Selection {
    use ashpd::desktop::{
        ResponseError,
        file_chooser::{FileFilter, SelectedFiles},
    };
    let response = SelectedFiles::open_file()
        .title("Import structured logs")
        .multiple(true)
        .filter(
            FileFilter::new("Log files")
                .glob("*.json")
                .glob("*.jsonl")
                .glob("*.ndjson"),
        )
        .send()
        .await
        .and_then(|request| request.response());
    match response {
        Ok(files) => files
            .uris()
            .iter()
            .map(|uri| {
                uri.to_file_path()
                    .map_err(|_| format!("Not a local file: {uri}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Err(ashpd::Error::Response(ResponseError::Cancelled)) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn open() -> Dialog {
    // Start on the UI thread so Cocoa can present the dialog safely.
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Import structured logs")
        .add_filter("Log files", &["json", "jsonl", "ndjson"])
        .pick_files();
    Box::pin(async move {
        Ok(dialog.await.map(|files| {
            files
                .into_iter()
                .map(|file| file.path().to_owned())
                .collect()
        }))
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn open() -> Dialog {
    Box::pin(async { Err("Native file dialogs are not supported on this platform".into()) })
}
