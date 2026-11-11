use super::branding::logo;
use super::{controls::*, theme::*};
use crate::{
    app::State,
    log,
    query::{InspectorTab, SLOW_THRESHOLD_MS},
};
use rune::tok::TextStyle;
use rune::{Color, El, ScrollSpec, TextEdit, col, row, text};

pub(super) fn welcome(s: &State) -> El {
    let mut content = col()
        .grow(1.0)
        .center()
        .gap(18.0)
        .child(logo(72.0).key("welcome-logo"))
        .child(label("A CLOSER LOOK AT YOUR LOGS", 11.0, accent()))
        .child(heading("Clarity in every event.", 36.0))
        .child(label(
            "Find failed requests, investigate slow responses, and follow the context.",
            14.0,
            muted(),
        ))
        .child(
            col()
                .key("drop-zone")
                .w(650.0)
                .h(210.0)
                .center()
                .gap(15.0)
                .radius(12.0)
                .fill(if s.drop_over {
                    active_surface()
                } else {
                    panel()
                })
                .border(1.0, if s.drop_over { accent() } else { line() })
                .child(mono("{ ··· }", 28.0, mint()))
                .child(heading("Drop your log files here", 18.0))
                .child(label(
                    "JSON, JSONL, or NDJSON · import multiple files together",
                    12.0,
                    muted(),
                ))
                .child(import_button().key("import-welcome")),
        )
        .child(
            row()
                .gap(10.0)
                .child(button("sample", "Explore sample logs", true).on_press(State::sample)),
        );
    if s.has_saved_session() {
        content = content.child(
            row()
                .gap(10.0)
                .child(button("restore", "Restore saved session", false).on_press(State::restore))
                .child(
                    button("delete-session", "Delete saved session", false)
                        .on_press(State::delete_session),
                ),
        );
    }
    content.child(label(
        "No uploads. No accounts. Sessions are saved only when you choose.",
        11.0,
        muted(),
    ))
}
fn metric(title: &str, value: String, color: Color) -> El {
    col()
        .grow(1.0)
        .h(90.0)
        .padding(14.0)
        .gap(10.0)
        .radius(7.0)
        .fill(panel())
        .border(1.0, line())
        .child(label(title, 11.0, muted()))
        .child(text(value).font(TextStyle::new(28.0, 500.0)).color(color))
}
pub(super) fn overview(s: &State, ids: &[usize]) -> El {
    let stats = s
        .results()
        .map(|result| result.stats.clone())
        .unwrap_or_default();
    col()
        .padding(22.0)
        .gap(14.0)
        .child(
            row()
                .justify_between()
                .center_y()
                .child(heading("Overview", 15.0))
                .child(label(
                    format!(
                        "{} {} / {} imported events",
                        s.workspace().files().len(),
                        if s.workspace().files().len() == 1 {
                            "file"
                        } else {
                            "files"
                        },
                        s.workspace().events().len()
                    ),
                    11.0,
                    muted(),
                )),
        )
        .child(
            row()
                .gap(10.0)
                .child(metric("Matching events", ids.len().to_string(), ink()))
                .child(
                    metric("Errors", stats.errors.to_string(), error_color())
                        .key("metric-errors")
                        .role("button")
                        .focusable(true)
                        .on_press(|s: &mut State| {
                            s.reset();
                            s.filters.level = "errors".into();
                        }),
                )
                .child(metric(
                    "Warnings",
                    stats.warnings.to_string(),
                    warning_color(),
                ))
                .child(metric("Requests", stats.requests.to_string(), ink()))
                .child(metric(
                    &format!("Slow ≥ {}", log::duration_label(Some(SLOW_THRESHOLD_MS))),
                    stats.slow.to_string(),
                    ink(),
                ))
                .child(metric(
                    "Request p95",
                    log::duration_label(stats.p95),
                    accent(),
                )),
        )
}
fn advanced(s: &State) -> El {
    let mut levels = row().gap(5.0).child(label("Level", 11.0, muted()).w(65.0));
    for level in [
        "", "trace", "debug", "info", "warn", "error", "fatal", "unknown",
    ] {
        levels = levels.child(
            button(
                format!("level-{level}"),
                if level.is_empty() { "All" } else { level },
                s.filters.level == level,
            )
            .on_press(move |s: &mut State| {
                s.filters.level = level.into();
            }),
        );
    }
    let mut methods = row().gap(5.0).child(label("Method", 11.0, muted()).w(65.0));
    for method in ["", "GET", "POST", "PUT", "PATCH", "DELETE"] {
        methods = methods.child(
            button(
                format!("method-{method}"),
                if method.is_empty() { "All" } else { method },
                s.filters.method == method,
            )
            .on_press(move |s: &mut State| {
                s.filters.method = method.into();
            }),
        );
    }
    let mut statuses = row()
        .gap(5.0)
        .child(label("HTTP status", 11.0, muted()).w(65.0));
    for status in [0, 2, 3, 4, 5] {
        statuses = statuses.child(
            button(
                format!("status-{status}"),
                if status == 0 {
                    "All".into()
                } else {
                    format!("{status}xx")
                },
                s.filters.status == status,
            )
            .on_press(move |s: &mut State| {
                s.filters.status = status;
            }),
        );
    }
    col()
        .gap(9.0)
        .padding(14.0)
        .fill(panel())
        .radius(7.0)
        .child(levels)
        .child(methods)
        .child(statuses)
        .child(
            row()
                .gap(10.0)
                .child(label("Interval", 11.0, muted()).w(60.0))
                .child(
                    input(
                        "start",
                        &s.start,
                        "Start · 2026-11-06T09:00:00Z",
                        |s: &mut State| &mut s.start,
                    )
                    .grow(1.0),
                )
                .child(
                    input(
                        "end",
                        &s.end,
                        "End · 2026-11-06T10:00:00Z",
                        |s: &mut State| &mut s.end,
                    )
                    .grow(1.0),
                ),
        )
        .child(
            row()
                .gap(9.0)
                .child(
                    input(
                        "view-name",
                        &s.view_name,
                        "Name this view",
                        |s: &mut State| &mut s.view_name,
                    )
                    .w(230.0),
                )
                .child(button("save-view", "Save view", false).on_press(State::save_view))
                .child(label("Saves filters on this device", 11.0, muted())),
        )
}
pub(super) fn toolbar(s: &State, error: Option<&String>) -> El {
    let time = match s.filters.minutes {
        15 => "Last 15 min",
        60 => "Last hour",
        1440 => "Last 24 hours",
        _ => "All time",
    };
    let mut out = col()
        .px(22.0)
        .pb(14.0)
        .gap(10.0)
        .child(
            row()
                .gap(8.0)
                .child(
                    input(
                        "search",
                        &s.search,
                        "Search logs… or status:503 duration>1000",
                        |s: &mut State| &mut s.search,
                    )
                    .grow(1.0),
                )
                .child(button("time-range", time, s.filters.minutes != 0).on_press(
                    |s: &mut State| {
                        s.filters.minutes = match s.filters.minutes {
                            0 => 15,
                            15 => 60,
                            60 => 1440,
                            _ => 0,
                        };
                    },
                ))
                .child(
                    button("filters", "Filters & views", s.advanced)
                        .on_press(|s: &mut State| s.advanced = !s.advanced),
                ),
        )
        .child(
            row()
                .gap(6.0)
                .flex_wrap()
                .center_y()
                .child(label("Try:", 11.0, muted()))
                .children(
                    [
                        ("search-errors", "status:503"),
                        ("search-slow", "duration>1000"),
                        ("search-phrase", "\"connection refused\""),
                    ]
                    .into_iter()
                    .map(|(key, query)| {
                        button(key, query, false)
                            .help("Replace search with this example; other filters stay active")
                            .on_press(move |s: &mut State| s.search = TextEdit::new(query))
                    }),
                )
                .child(
                    button("utc", if s.utc { "UTC" } else { "Local time" }, false)
                        .on_press(|s: &mut State| s.utc = !s.utc),
                ),
        );
    if s.advanced {
        out = out.child(advanced(s));
    }
    if s.query_active() {
        let mut chips = Vec::new();
        let mut add = |key: &str, title: String, remove: fn(&mut State)| {
            chips.push(
                button(key, format!("{title} ×"), true)
                    .max_w(260.0)
                    .label(format!("Remove filter: {title}"))
                    .help(&title)
                    .on_press(remove),
            );
        };
        if !s.search.text.is_empty() {
            add("chip-search", format!("Search: {}", s.search.text), |s| {
                s.search = TextEdit::new("")
            });
        }
        if let Some(file) = s.filters.file.and_then(|i| s.workspace().files().get(i)) {
            add("chip-file", format!("File: {file}"), |s| {
                s.filters.file = None
            });
        }
        if !s.filters.level.is_empty() {
            add("chip-level", format!("Level: {}", s.filters.level), |s| {
                s.filters.level.clear()
            });
        }
        if !s.filters.method.is_empty() {
            add(
                "chip-method",
                format!("Method: {}", s.filters.method),
                |s| s.filters.method.clear(),
            );
        }
        if s.filters.status != 0 {
            add(
                "chip-status",
                format!(
                    "Status: {}{}",
                    s.filters.status,
                    if s.filters.status < 100 { "xx" } else { "" }
                ),
                |s| s.filters.status = 0,
            );
        }
        if s.filters.slow {
            add("chip-slow", "Slow requests".into(), |s| {
                s.filters.slow = false
            });
        }
        if s.filters.minutes != 0 {
            add("chip-time", time.into(), |s| s.filters.minutes = 0);
        }
        if !s.start.text.is_empty() {
            add("chip-start", format!("From: {}", s.start.text), |s| {
                s.start = TextEdit::new("")
            });
        }
        if !s.end.text.is_empty() {
            add("chip-end", format!("To: {}", s.end.text), |s| {
                s.end = TextEdit::new("")
            });
        }
        if let Some(correlation) = &s.correlation {
            add(
                "chip-correlation",
                format!("{}: {}", correlation.field, correlation.value),
                |s| s.correlation = None,
            );
        }
        out = out.child(
            row()
                .key("active-filters")
                .gap(6.0)
                .flex_wrap()
                .center_y()
                .children(chips)
                .child(button("clear-filters", "Clear filters", false).on_press(State::reset)),
        );
    }
    if let Some(error) = error {
        out = out.child(
            label(error, 12.0, error_color())
                .key("search-error")
                .role("alert"),
        );
    }
    out
}
const TIME_WIDTH: f32 = 104.0;
const LEVEL_WIDTH: f32 = 57.0;
const METHOD_WIDTH: f32 = 46.0;
const STATUS_WIDTH: f32 = 38.0;
const DURATION_WIDTH: f32 = 64.0;
fn table_row(s: &State, i: usize) -> El {
    let e = &s.workspace().events()[i];
    let level = e.level();
    let status = e.status();
    let path = e.field("path");
    let filter_level = level.to_owned();
    let mut status_cell = mono(
        status.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
        11.0,
        if status.is_some_and(|n| n >= 500) {
            error_color()
        } else {
            muted()
        },
    )
    .w(STATUS_WIDTH);
    if let Some(status) = status {
        status_cell = status_cell
            .key(format!("filter-status-{i}"))
            .role("button")
            .label(format!("Filter status {status}"))
            .focusable(true)
            .hover(active_surface())
            .on_press(move |s: &mut State| s.filters.status = status);
    }
    row()
        .key(format!("event-{i}"))
        .role("row")
        .label(format!(
            "{} {} {}",
            e.time(s.utc),
            level,
            e.field("message")
        ))
        .focusable(true)
        .h(62.0)
        .px(16.0)
        .gap(10.0)
        .center_y()
        .border(0.5, line())
        .fill(if s.selected == Some(i) {
            selected_surface()
        } else {
            bg()
        })
        .hover(Color::WHITE.alpha(0.03))
        .child(mono(e.time(s.utc), 11.0, muted()).w(TIME_WIDTH))
        .child(
            badge(&level.to_uppercase(), level_color(level), LEVEL_WIDTH)
                .key(format!("filter-level-{i}"))
                .role("button")
                .label(format!("Filter level {level}"))
                .focusable(true)
                .hover(active_surface())
                .on_press(move |s: &mut State| s.filters.level = filter_level.clone()),
        )
        .child(mono(e.field("method"), 11.0, accent()).w(METHOD_WIDTH))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(4.0)
                .child(
                    mono(
                        if path.is_empty() {
                            e.field("message")
                        } else {
                            path
                        },
                        11.0,
                        ink(),
                    )
                    .w_full()
                    .truncate(),
                )
                .child(label(e.field("message"), 11.0, muted()).w_full().truncate()),
        )
        .child(status_cell)
        .child(
            mono(
                log::duration_label(e.duration()),
                11.0,
                if e.duration().is_some_and(|d| d >= SLOW_THRESHOLD_MS) {
                    warning_color()
                } else {
                    muted()
                },
            )
            .w(DURATION_WIDTH),
        )
        .on_press(move |s: &mut State| {
            s.selected = Some(i);
            s.tab = InspectorTab::Fields;
        })
}
pub(super) fn table(s: &State, ids: &[usize]) -> El {
    let last_page = ids.len().saturating_sub(1) / 100;
    let page = s.table_page().min(last_page);
    let mut rows = col();
    for i in ids.iter().skip(page * 100).take(100) {
        rows = rows.child(table_row(s, *i));
    }
    if ids.is_empty() {
        rows = rows.child(
            col()
                .h(220.0)
                .center()
                .gap(12.0)
                .child(heading("No matching events", 18.0))
                .child(label(
                    "Try changing your search or clearing the filters.",
                    12.0,
                    muted(),
                ))
                .child(button("empty-reset", "Clear filters", false).on_press(State::reset)),
        );
    }
    col()
        .grow(1.0)
        .min_w(0.0)
        .min_h(0.0)
        .child(
            row()
                .h(45.0)
                .px(18.0)
                .center_y()
                .justify_between()
                .border(0.5, line())
                .child(
                    label(
                        format!(
                            "{} matching {}",
                            ids.len(),
                            if ids.len() == 1 { "event" } else { "events" }
                        ),
                        12.0,
                        muted(),
                    )
                    .key("matching-count"),
                )
                .child(
                    row()
                        .gap(8.0)
                        .child(
                            button(
                                "sort",
                                if s.ascending {
                                    "Oldest first ↑"
                                } else {
                                    "Newest first ↓"
                                },
                                false,
                            )
                            .on_press(|s: &mut State| {
                                s.ascending = !s.ascending;
                            }),
                        )
                        .child(export_button()),
                ),
        )
        .child(
            row()
                .h(32.0)
                .px(16.0)
                .gap(10.0)
                .center_y()
                .fill(panel())
                .child(label("TIME", 11.0, muted()).w(TIME_WIDTH))
                .child(label("LEVEL", 11.0, muted()).w(LEVEL_WIDTH))
                .child(label("METHOD", 11.0, muted()).w(METHOD_WIDTH))
                .child(label("EVENT / PATH", 11.0, muted()).grow(1.0))
                .child(label("STATUS", 11.0, muted()).w(STATUS_WIDTH))
                .child(label("DURATION", 11.0, muted()).w(DURATION_WIDTH)),
        )
        .child(
            col()
                .key("event-scroll")
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(rows),
        )
        .child(
            row()
                .h(38.0)
                .px(16.0)
                .gap(8.0)
                .center_y()
                .justify_between()
                .child(label(
                    format!(
                        "Page {} of {} · up to 100 events per page",
                        page + 1,
                        ids.len().div_ceil(100).max(1)
                    ),
                    11.0,
                    muted(),
                ))
                .child(
                    row()
                        .gap(6.0)
                        .child(
                            button("prev-page", "←", false)
                                .on_press(move |s: &mut State| s.set_page(page.saturating_sub(1))),
                        )
                        .child(
                            button("next-page", "→", false).on_press(move |s: &mut State| {
                                s.set_page((page + 1).min(last_page))
                            }),
                        ),
                ),
        )
}
