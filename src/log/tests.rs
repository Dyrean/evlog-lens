use super::*;
#[test]
fn parses_formats_and_retains_source_errors() {
    let b = parse("a.ndjson", 2, "\u{feff}{\"msg\":\"ok\"}\nnope\n42\n{}\n");
    assert_eq!(b.events.len(), 2);
    assert_eq!(b.errors.len(), 2);
    assert_eq!(b.events[1].line(), Some(4));
    assert_eq!(b.events[0].source(), 2);
    assert_eq!(
        parse("a.json", 0, "{\"events\":[{}, {}, 4]}").events.len(),
        2
    );
    assert_eq!(parse("a.json", 0, "[\n{}\nbad\n]").events.len(), 0);
}
#[test]
fn normalizes_aliases_and_units_without_changing_raw() {
    let b = parse(
        "x.json",
        0,
        r#"{"time":1700000000,"severity":"WARNING","request":{"method":"get","id":"r"},"status":"success","response":{"status":503},"duration":"1.2s","context":{"session":{"id":"s"}}}"#,
    );
    let e = &b.events[0];
    assert_eq!(e.level(), "warn");
    assert_eq!(e.status(), Some(503));
    assert_eq!(e.duration(), Some(1200.0));
    assert_eq!(e.timestamp(), Some(1700000000000));
    assert_eq!(e.field("session"), "s");
    assert_eq!(e.raw()["duration"], "1.2s");
}
#[test]
fn searches_exact_nested_numeric_quoted_and_fuzzy() {
    let b = parse(
        "a.json",
        0,
        r#"{"status":503,"duration":3200,"message":"payment provider failed","user":{"id":"U-1"}}"#,
    );
    let e = &b.events[0];
    assert!(matches(
        e,
        &search_terms("status:503 duration>=3000 user.id:u-1 \"payment provider\"").unwrap()
    ));
    assert!(!matches(e, &search_terms("status:50").unwrap()));
    assert!(matches(e, &search_terms("pymnt").unwrap()));
    assert!(search_terms("duration>oops").is_err());
    assert!(search_terms("\"open").is_err());
}
#[test]
fn session_aliases_skip_invalid_candidates_and_survive_serialization() {
    for invalid in ["{}", "[]", "42", "true", r#""   ""#] {
        let input = format!(r#"{{"session":{{"id":{invalid}}},"sid":"valid-session"}}"#);
        let event = parse("session.json", 0, &input).events.remove(0);
        assert_eq!(event.field("session"), "valid-session");
        assert!(matches(
            &event,
            &search_terms("session:valid-session").unwrap()
        ));
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(!serialized.contains("cached"));
        let restored: Event = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored.field("session"), "valid-session");
    }
}
#[test]
fn cancellation_interrupts_ndjson_and_a_single_large_json_document() {
    use std::cell::Cell;
    for (file, input) in [
        ("large.ndjson", "{\"message\":\"event\"}\n".repeat(10000)),
        (
            "large.json",
            format!(r#"{{"message":"{}"}}"#, "x".repeat(200000)),
        ),
        (
            "large.json",
            format!("[{}]", vec![r#"{"message":"event"}"#; 10000].join(",")),
        ),
    ] {
        let checks = Cell::new(0);
        let result = parse_cancellable(file, 0, &input, &|| {
            checks.set(checks.get() + 1);
            checks.get() >= 10
        });
        assert!(result.is_none(), "{file} must discard a cancelled parse");
    }
}
