use super::*;
use std::{
    collections::HashMap,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

struct Portal(AtomicUsize);

#[zbus::interface(name = "org.freedesktop.portal.FileChooser")]
impl Portal {
    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        3
    }

    async fn open_file(
        &self,
        _parent: &str,
        _title: &str,
        options: HashMap<String, OwnedValue>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        assert!(bool::try_from(options.get("multiple").unwrap()).unwrap());
        let token = <&str>::try_from(options.get("handle_token").unwrap()).unwrap();
        let sender = header
            .sender()
            .unwrap()
            .trim_start_matches(':')
            .replace('.', "_");
        let path = OwnedObjectPath::try_from(format!(
            "/org/freedesktop/portal/desktop/request/{sender}/{token}"
        ))
        .unwrap();
        let call = self.0.fetch_add(1, Ordering::Relaxed);
        let uris = if call == 3 {
            vec!["https://example.com/not-a-local-file"]
        } else {
            vec!["file:///tmp/first%20log.ndjson", "file:///tmp/second.jsonl"]
        };
        let results = HashMap::from([("uris", OwnedValue::try_from(Value::from(uris)).unwrap())]);
        connection
            .emit_signal(
                None::<&str>,
                path.as_str(),
                "org.freedesktop.portal.Request",
                "Response",
                &(if call == 2 { 1u32 } else { 0u32 }, results),
            )
            .await?;
        Ok(path)
    }
}

fn finish(mut dialog: Dialog) -> Selection {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Poll::Ready(selection) = dialog
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            return selection;
        }
        assert!(Instant::now() < deadline, "File picker did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn linux_picker_can_be_reopened_and_handles_portal_responses() {
    // A subprocess keeps Ashpd's cached connection and the fake service off the user's bus.
    if std::env::var_os("EVLOG_TEST_PRIVATE_PORTAL").is_none() {
        let output = Command::new("dbus-run-session")
            .arg("--")
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "file_picker::tests::linux_picker_can_be_reopened_and_handles_portal_responses",
                "--nocapture",
            ])
            .env("EVLOG_TEST_PRIVATE_PORTAL", "1")
            .output();
        let output = match output {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("Skipping private portal test: dbus-run-session is not installed");
                return;
            }
            Err(error) => panic!("Could not start private portal test: {error}"),
        };
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let _portal = zbus::blocking::connection::Builder::session()
        .unwrap()
        .name("org.freedesktop.portal.Desktop")
        .unwrap()
        .serve_at(
            "/org/freedesktop/portal/desktop",
            Portal(AtomicUsize::new(0)),
        )
        .unwrap()
        .build()
        .unwrap();
    for _ in 0..2 {
        assert_eq!(
            finish(open()).unwrap().unwrap(),
            [
                PathBuf::from("/tmp/first log.ndjson"),
                PathBuf::from("/tmp/second.jsonl"),
            ]
        );
    }
    assert_eq!(finish(open()).unwrap(), None);
    assert!(finish(open()).unwrap_err().contains("Not a local file"));
}
