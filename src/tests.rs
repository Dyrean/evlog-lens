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
    runtime_at((1480.0, 940.0))
}
fn runtime_at(size: (f32, f32)) -> (Runtime<State>, TempDir) {
    let directory = TempDir::new();
    let root = directory.path().to_owned();
    let mut rt = App::new(shell::spec())
        .init(move || State::new(Store::new(root)))
        .headless_runtime(size, Env::new(true, 1.0));
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
fn multi_file_import_and_export_dialogs_use_the_real_poll_hook() {
    let (mut rt, directory) = runtime();
    let fixture = directory.write(
        "events.ndjson",
        include_str!("../examples/sample-evlog.ndjson"),
    );
    let second = directory.write("worker.jsonl", "{\"message\":\"worker event\"}\n");
    rt.update(move |s, _| {
        let paths = vec![fixture, second];
        s.import_picker = Box::new(move || Box::pin(std::future::ready(Ok(Some(paths.clone())))));
    });
    click(&mut rt, "import");
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
    assert!(text_visible(&rt, "51 matching events"));
    assert_eq!(rt.state.workspace().files().len(), 2);
    assert_eq!(rt.state.workspace().events()[50].source(), 1);
    click(&mut rt, "export");
    assert!(rt.file_dialog().is_some());
    let output = directory.path().join("matching.ndjson");
    rt.resolve_file_dialog(Some(output.clone())).unwrap();
    settle(&mut rt);
    assert_eq!(std::fs::read_to_string(output).unwrap().lines().count(), 51);
}
#[test]
fn cancelling_or_failing_the_import_picker_keeps_loaded_logs() {
    let (mut rt, _directory) = runtime();
    click(&mut rt, "sample");
    for selection in [Ok(None), Ok(Some(vec![])), Err("portal unavailable".into())] {
        rt.update(move |s, _| {
            s.import_picker = Box::new(move || Box::pin(std::future::ready(selection.clone())));
        });
        click(&mut rt, "import");
        assert!(!rt.state.choosing_import_files());
        assert!(!rt.state.importing());
        assert_eq!(rt.state.workspace().events().len(), 50);
    }
    assert!(rt.state.notice.contains("portal unavailable"));
}
#[test]
fn repeated_import_clicks_do_not_open_more_pickers() {
    let (mut rt, _directory) = runtime();
    let opened = std::rc::Rc::new(std::cell::Cell::new(0));
    let counter = opened.clone();
    rt.update(move |s, _| {
        s.import_picker = Box::new(move || {
            counter.set(counter.get() + 1);
            Box::pin(std::future::pending())
        });
    });
    click(&mut rt, "import");
    click(&mut rt, "import");
    assert!(rt.state.choosing_import_files());
    assert_eq!(opened.get(), 1);
}
#[test]
fn clicking_event_values_filters_without_opening_the_inspector() {
    let (mut rt, _directory) = runtime();
    click(&mut rt, "sample");
    let index = rt
        .state
        .workspace()
        .events()
        .iter()
        .position(|e| e.status() == Some(503))
        .unwrap();
    click(&mut rt, &format!("filter-status-{index}"));
    assert_eq!(rt.state.filters.status, 503);
    assert_eq!(rt.state.selected, None);
    assert!(text_visible(&rt, "1 matching event"));
    click(&mut rt, &format!("filter-level-{index}"));
    assert_eq!(rt.state.filters.level, "error");
    assert_eq!(rt.state.filters.status, 503);
    click(&mut rt, "chip-status");
    assert_eq!(rt.state.filters.status, 0);
    assert_eq!(rt.state.filters.level, "error");
    click(&mut rt, "chip-level");
    assert!(text_visible(&rt, "50 matching events"));
    click(&mut rt, "search-errors");
    assert_eq!(rt.state.search.text, "status:503");
    assert!(text_visible(&rt, "1 matching event"));
    click(&mut rt, "chip-search");
    click(&mut rt, "search-slow");
    assert_eq!(rt.state.search.text, "duration>1000");
    assert!(!rt.state.results().unwrap().ids.is_empty());
}
#[test]
fn every_filter_chip_clears_only_its_own_filter() {
    use crate::query::{Correlation, CorrelationField};
    let (mut rt, _directory) = runtime();
    click(&mut rt, "sample");
    rt.update(|s, _| {
        s.search = TextEdit::new("status:503");
        s.filters = Filters {
            file: Some(0),
            level: "error".into(),
            method: "POST".into(),
            status: 5,
            slow: true,
            minutes: 60,
        };
        s.start = TextEdit::new("2026-11-06T09:00:00Z");
        s.end = TextEdit::new("2026-11-06T10:00:00Z");
        s.correlation = Some(Correlation {
            field: CorrelationField::Session,
            value: "sess-local-001".into(),
        });
    });
    settle(&mut rt);
    type RemoveFilter = fn(&mut Query);
    let removals: [(&str, RemoveFilter); 10] = [
        ("chip-search", |q| q.query.clear()),
        ("chip-file", |q| q.filters.file = None),
        ("chip-level", |q| q.filters.level.clear()),
        ("chip-method", |q| q.filters.method.clear()),
        ("chip-status", |q| q.filters.status = 0),
        ("chip-slow", |q| q.filters.slow = false),
        ("chip-time", |q| q.filters.minutes = 0),
        ("chip-start", |q| q.start.clear()),
        ("chip-end", |q| q.end.clear()),
        ("chip-correlation", |q| q.correlation = None),
    ];
    for (key, remove) in removals {
        let mut expected = rt.state.query();
        remove(&mut expected);
        click(&mut rt, key);
        assert_eq!(rt.state.query(), expected, "{key}");
        assert!(rt.find(key).is_none());
    }
    assert!(text_visible(&rt, "50 matching events"));
}
#[test]
fn expanded_filters_leave_room_for_events_in_a_compact_window() {
    let (mut rt, directory) = runtime_at((1180.0, 720.0));
    let path = directory.write("bad.ndjson", "bad\n");
    rt.update(move |s, _| {
        s.sample();
        s.load_files(&[path]);
        s.advanced = true;
        s.show_errors = true;
        s.selected = Some(0);
        s.search = TextEdit::new("status:503");
        s.filters = Filters {
            file: Some(0),
            level: "error".into(),
            method: "POST".into(),
            status: 5,
            slow: true,
            minutes: 60,
        };
        s.start = TextEdit::new("2026-11-06T09:00:00Z");
        s.end = TextEdit::new("2026-11-06T10:00:00Z");
    });
    settle(&mut rt);
    let scroll = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("event-scroll"))
        .unwrap()
        .rect;
    assert!(scroll[3] >= 62.0, "No room for even one event: {scroll:?}");
    assert!(scroll[1] + scroll[3] <= 720.0);
    rt.update(|_, tx| tx.scroll_to_bottom_instant("filter-scroll"));
    settle(&mut rt);
    let rect = |key: &str| {
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap()
            .rect
    };
    let filters = rect("filter-scroll");
    let clear = rect("clear-filters");
    assert!(clear[1] >= filters[1] && clear[1] + clear[3] <= filters[1] + filters[3]);
    click(&mut rt, "clear-filters");
    assert_eq!(rt.state.query(), Query::default());
}
#[test]
fn source_details_and_removal_work_through_sidebar_controls() {
    let (mut rt, directory) = runtime();
    let first = directory.write(
        "first.ndjson",
        "{\"message\":\"first\",\"timestamp\":1}\nbad\n",
    );
    let second = directory.write("second.ndjson", "{\"message\":\"second\"}\nbad\n");
    rt.update(move |s, _| s.load_files(&[first, second]));
    settle(&mut rt);
    assert!(text_visible(&rt, "1 event · 1 import issue"));
    assert!(text_visible(&rt, "From 1970-01-01 00:00:01"));
    assert!(text_visible(&rt, "No dated events"));
    click(&mut rt, "file-1");
    click(&mut rt, "remove-file-0");
    assert_eq!(rt.state.filters.file, Some(0));
    assert_eq!(rt.state.workspace().files(), ["second.ndjson"]);
    assert_eq!(rt.state.workspace().errors().len(), 1);
    assert!(directory.path().join("first.ndjson").is_file());
    click(&mut rt, "remove-file-0");
    assert!(rt.state.workspace().events().is_empty());
    assert!(rt.state.workspace().errors().is_empty());
    assert_eq!(rt.state.filters.file, None);
    assert!(rt.find("import-welcome").is_some());
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
