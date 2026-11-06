use crate::{
    app::{self, State},
    query::InspectorTab,
    storage::Store,
    ui,
};
use rune::{App, AppSpec, Key, KeyEvent, Tx};
use std::path::PathBuf;

#[derive(Default)]
struct Startup {
    sample: bool,
    files: Vec<PathBuf>,
    logo: Option<PathBuf>,
}
impl Startup {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut startup = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--sample" => startup.sample = true,
                "--load" | "--export-logo" => {
                    let value = args
                        .next()
                        .filter(|value| !value.starts_with("--"))
                        .ok_or_else(|| format!("{arg} requires a file path"))?;
                    if arg == "--load" {
                        startup.files.push(value.into());
                    } else {
                        startup.logo = Some(value.into());
                    }
                }
                _ => return Err(format!("Unknown application argument: {arg}")),
            }
        }
        Ok(startup)
    }
    fn initialize(self, store: Store) -> State {
        let mut state = State::new(store);
        let preferences_notice = state.notice.clone();
        if self.sample {
            state.sample();
        }
        if !self.files.is_empty() {
            state.load_files(&self.files);
        }
        if !preferences_notice.is_empty() {
            state.notice = if state.notice.is_empty() {
                preferences_notice
            } else {
                format!("{} · {preferences_notice}", state.notice)
            };
        }
        state
    }
}
fn on_key(state: &mut State, event: &KeyEvent, _: &mut Tx) -> bool {
    if event.key == Key::Escape {
        state.selected = None;
        true
    } else {
        false
    }
}
fn incident(state: &mut State) {
    state.sample();
    state.selected = state
        .workspace()
        .events()
        .iter()
        .position(|event| event.status() == Some(503));
}
pub fn spec() -> AppSpec<State> {
    AppSpec {
        min_size: Some((1180.0, 720.0)),
        poll: Some(app::poll),
        on_key: Some(on_key),
        scenes: &[
            ("sample", State::sample),
            ("incident", incident),
            ("errors", |s| {
                s.sample();
                s.filters.level = "errors".into();
            }),
            ("session", |s| {
                incident(s);
                s.tab = InspectorTab::Session;
            }),
            ("json", |s| {
                incident(s);
                s.tab = InspectorTab::Json;
            }),
        ],
        families: ("Inter", "IBM Plex Mono"),
        ..AppSpec::new("Evlog Lens", (1480.0, 940.0), State::default, ui::view)
    }
}
pub fn run() {
    let startup = match Startup::parse(rune::extra_args().iter().cloned()) {
        Ok(startup) => startup,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    if let Some(path) = &startup.logo {
        if let Err(error) = ui::export_logo(path) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    App::new(spec())
        .init(move || startup.initialize(Store::discover()))
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    #[test]
    fn startup_uses_the_same_import_and_metadata_rules_as_background_work() {
        let dir = TempDir::new();
        let one = dir.write("one.ndjson", "{\"message\":\"one\"}\nbroken\n");
        let two = dir.write("two.json", "{\"message\":\"two\"}");
        let startup = Startup::parse([
            "--sample".into(),
            "--load".into(),
            one.display().to_string(),
            "--load".into(),
            two.display().to_string(),
        ])
        .unwrap();
        let state = startup.initialize(Store::new(dir.path().join("saved")));
        assert_eq!(state.workspace().events().len(), 52);
        assert_eq!(state.workspace().events()[50].source(), 1);
        assert_eq!(state.workspace().events()[51].source(), 2);
        assert_eq!(state.workspace().errors().len(), 1);
    }
    #[test]
    fn malformed_startup_arguments_are_reported() {
        assert!(Startup::parse(["--load".into()]).is_err());
        assert!(Startup::parse(["--load".into(), "--sample".into()]).is_err());
        assert!(Startup::parse(["--unknown".into()]).is_err());
    }
}
