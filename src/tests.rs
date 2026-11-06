use crate::{
    app::State,
    query::{Filters, Query, SavedView},
    shell,
    storage::Store,
    test_support::TempDir,
};
use rune::{App, Env, Key, KeyEvent, Mods, Runtime, TextEdit};
use std::time::{Duration, Instant};

fn runtime() -> (Runtime<State>, TempDir) {
    let directory = TempDir::new();
    let root = directory.path().to_owned();
    let mut rt = App::new(shell::spec())
        .init(move || State::new(Store::new(root)))
        .headless_runtime((1480.0, 940.0), Env::new(true, 1.0));
    rt.frame(0.0);
    (rt, directory)
}
fn settle(rt: &mut Runtime<State>) {
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}
fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt
        .find(key)
        .unwrap_or_else(|| panic!("missing element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    settle(rt);
}
fn text_visible(rt: &Runtime<State>, needle: &str) -> bool {
    rt.elements()
        .iter()
        .any(|e| e.text.as_deref().is_some_and(|s| s.contains(needle)))
}

#[test]
fn sample_filters_inspector_and_timelines_work_through_production_hooks() {
    let (mut rt, _directory) = runtime();
    click(&mut rt, "sample");
    assert!(text_visible(&rt, "50 matching events"));
    click(&mut rt, "errors");
    assert!(text_visible(&rt, "3 matching events"));
    let i = rt.state.results().unwrap().ids[0];
    click(&mut rt, &format!("event-{i}"));
    assert_eq!(rt.state.selected, Some(i));
    click(&mut rt, "tab-json");
    assert!(rt.find("json-/error").is_some());
    click(&mut rt, "toggle-json-/error");
    assert!(rt.find("json-/error").is_none());
    click(&mut rt, "tab-timeline");
    assert!(text_visible(&rt, "Request / trace events · chronological"));
    click(&mut rt, "tab-session");
    assert!(text_visible(
        &rt,
        "Requests in this session · chronological"
    ));
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    settle(&mut rt);
    assert!(rt.find("inspector").is_none());
    click(&mut rt, "clear-filters");
    assert!(text_visible(&rt, "50 matching events"));
}
#[test]
fn native_text_input_matches_queries_and_reports_invalid_queries() {
    let (mut rt, _directory) = runtime();
    click(&mut rt, "sample");
    click(&mut rt, "search");
    rt.paste("status:503 duration>1000");
    settle(&mut rt);
    assert!(text_visible(&rt, "1 matching event"));
    let ids = &rt.state.results().unwrap().ids;
    assert!(
        ids.iter()
            .all(|i| rt.state.workspace().events()[*i].status() == Some(503))
    );
    rt.update(|s, _| s.search = TextEdit::new("duration>oops"));
    settle(&mut rt);
    assert!(rt.find("search-error").is_some());
}
#[test]
fn native_import_and_export_dialogs_use_the_real_poll_hook() {
    let (mut rt, directory) = runtime();
    let fixture = directory.write(
        "events.ndjson",
        include_str!("../examples/sample-evlog.ndjson"),
    );
    click(&mut rt, "import");
    assert!(rt.file_dialog().is_some());
    rt.resolve_file_dialog(Some(fixture)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        rt.frame(1.0 / 60.0);
        if !rt.state.importing() && !rt.state.workspace().events().is_empty() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Production poll hook did not commit the import"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    settle(&mut rt);
    assert!(text_visible(&rt, "50 matching events"));
    click(&mut rt, "export");
    assert!(rt.file_dialog().is_some());
    let output = directory.path().join("matching.ndjson");
    rt.resolve_file_dialog(Some(output.clone())).unwrap();
    settle(&mut rt);
    assert_eq!(std::fs::read_to_string(output).unwrap().lines().count(), 50);
}
#[test]
fn overflowing_sidebar_sources_and_views_remain_reachable() {
    let (mut rt, directory) = runtime();
    let paths: Vec<_> = (0..30)
        .map(|i| directory.write(&format!("source-{i}.ndjson"), "{\"message\":\"event\"}\n"))
        .collect();
    rt.update(move |s, _| {
        s.load_files(&paths);
        s.views = vec![SavedView {
            name: "Saved incident".into(),
            filters: Query::default(),
        }];
    });
    settle(&mut rt);
    rt.update(|_, tx| tx.scroll_to_bottom_instant("sidebar-scroll"));
    settle(&mut rt);
    let rect = |key: &str| {
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap()
            .rect
    };
    let scroll = rect("sidebar-scroll");
    let file = rect("file-29");
    assert!(file[1] >= scroll[1] && file[1] + file[3] <= scroll[1] + scroll[3]);
    click(&mut rt, "file-29");
    assert!(text_visible(&rt, "1 matching event"));
    click(&mut rt, "view-0");
    assert!(text_visible(&rt, "30 matching events"));
    assert_eq!(rt.state.filters, Filters::default());
}
#[test]
fn import_diagnostics_do_not_hide_entries_after_one_hundred() {
    let (mut rt, directory) = runtime();
    let file = directory.write("broken.ndjson", "bad\n".repeat(105));
    rt.update(move |s, _| {
        s.load_files(&[file]);
        s.show_errors = true;
    });
    settle(&mut rt);
    assert!(text_visible(&rt, "broken.ndjson:105"));
}
