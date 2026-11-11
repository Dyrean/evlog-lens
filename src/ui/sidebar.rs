use super::branding::logo;
use super::{controls::*, theme::*};
use crate::{app::State, log, query::SLOW_THRESHOLD_MS};
use rune::{El, ScrollSpec, col, el, row};

pub(super) fn sidebar(s: &State) -> El {
    let summaries = s.workspace().file_summaries();
    let date = |value: Option<i64>| {
        value
            .and_then(chrono::DateTime::from_timestamp_millis)
            .map(|time| {
                if s.utc {
                    time.format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    time.with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                }
            })
            .unwrap_or_else(|| "—".into())
    };
    let mut files = col().gap(8.0);
    for (i, name) in s.workspace().files().iter().enumerate() {
        let summary = &summaries[i];
        files = files.child(
            col()
                .gap(3.0)
                .child(
                    row()
                        .gap(3.0)
                        .center_y()
                        .child(
                            nav(
                                format!("file-{i}"),
                                name.clone(),
                                summary.events.to_string(),
                                s.filters.file == Some(i),
                                move |s: &mut State| {
                                    s.filters.file = if s.filters.file == Some(i) {
                                        None
                                    } else {
                                        Some(i)
                                    };
                                },
                            )
                            .grow(1.0)
                            .min_w(0.0)
                            .gap(6.0)
                            .help(name.clone()),
                        )
                        .child(
                            button(format!("remove-file-{i}"), "×", false)
                                .label(format!("Remove {name} from workspace"))
                                .help("Remove from workspace only; original file is not deleted")
                                .on_press(move |s: &mut State| s.remove_file(i)),
                        ),
                )
                .child(
                    label(
                        format!(
                            "{} {} · {} import {}",
                            summary.events,
                            if summary.events == 1 {
                                "event"
                            } else {
                                "events"
                            },
                            summary.issues,
                            if summary.issues == 1 {
                                "issue"
                            } else {
                                "issues"
                            }
                        ),
                        10.0,
                        if summary.issues > 0 {
                            warning_color()
                        } else {
                            muted()
                        },
                    )
                    .key(format!("file-summary-{i}"))
                    .px(10.0),
                )
                .child(
                    col()
                        .px(10.0)
                        .gap(2.0)
                        .child(label(
                            if summary.first.is_some() {
                                format!("From {}", date(summary.first))
                            } else {
                                "No dated events".into()
                            },
                            10.0,
                            muted(),
                        ))
                        .child(label(
                            if summary.last.is_some() {
                                format!(
                                    "To {}{}",
                                    date(summary.last),
                                    if s.utc { " UTC" } else { " local" }
                                )
                            } else {
                                String::new()
                            },
                            10.0,
                            muted(),
                        )),
                ),
        );
    }
    let mut views = col().gap(3.0);
    for (i, v) in s.views.iter().enumerate() {
        views = views.child(
            row()
                .gap(4.0)
                .child(
                    nav(
                        format!("view-{i}"),
                        v.name.clone(),
                        "",
                        false,
                        move |s: &mut State| s.load_view(i),
                    )
                    .grow(1.0),
                )
                .child(
                    button(format!("delete-view-{i}"), "×", false)
                        .on_press(move |s: &mut State| {
                            s.delete_view(i);
                        })
                        .label(format!("Delete view {}", v.name)),
                ),
        );
    }
    let totals = s.workspace().totals();
    let sections = col()
        .gap(5.0)
        .child(label("WORKSPACE", 11.0, muted()).mt(12.0).mb(8.0))
        .child(nav(
            "all-events",
            "All events",
            s.workspace().events().len().to_string(),
            !s.query_active(),
            State::reset,
        ))
        .child(nav(
            "errors",
            "Errors & fatal",
            totals.errors.to_string(),
            s.filters.level == "errors",
            |s: &mut State| {
                s.reset();
                s.filters.level = "errors".into();
            },
        ))
        .child(nav(
            "warnings",
            "Warnings",
            totals.warnings.to_string(),
            s.filters.level == "warn",
            |s: &mut State| {
                s.reset();
                s.filters.level = "warn".into();
            },
        ))
        .child(nav(
            "slow",
            "Slow requests",
            format!("≥ {}", log::duration_label(Some(SLOW_THRESHOLD_MS))),
            s.filters.slow,
            |s: &mut State| {
                s.reset();
                s.filters.slow = true;
            },
        ))
        .child(nav(
            "server-errors",
            "Server errors",
            "5xx",
            s.filters.status == 5,
            |s: &mut State| {
                s.reset();
                s.filters.status = 5;
            },
        ))
        .child(label("SOURCE FILES", 11.0, muted()).mt(24.0).mb(8.0))
        .child(files)
        .child(
            label(
                if s.workspace().files().is_empty() {
                    "No files imported".into()
                } else {
                    format!(
                        "{} local {}",
                        s.workspace().files().len(),
                        if s.workspace().files().len() == 1 {
                            "source"
                        } else {
                            "sources"
                        }
                    )
                },
                11.0,
                muted(),
            )
            .px(10.0)
            .mt(5.0),
        )
        .child(label("SAVED VIEWS", 11.0, muted()).mt(24.0).mb(8.0))
        .child(views)
        .child(if s.views.is_empty() {
            label("Save a filter. Pick up later.", 11.0, muted()).px(10.0)
        } else {
            el()
        });
    col()
        .w(224.0)
        .h_full()
        .min_h(0.0)
        .fill(sidebar_surface())
        .border(1.0, line())
        .padding(16.0)
        .gap(12.0)
        .child(
            row().h(50.0).gap(10.0).center_y().child(logo(36.0)).child(
                col()
                    .gap(4.0)
                    .child(heading("Evlog Lens", 18.0))
                    .child(label("LOCAL LOG WORKSPACE", 11.0, muted())),
            ),
        )
        .child(rule())
        .child(
            col()
                .key("sidebar-scroll")
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(sections.pb(18.0)),
        )
        .child(button("sample-sidebar", "Explore sample logs", false).on_press(State::sample))
        .child(rule())
        .child(
            row()
                .gap(7.0)
                .center_y()
                .child(el().size(6.0, 6.0).radius(3.0).fill(mint()))
                .child(label("Private by design", 11.0, ink())),
        )
        .child(label("Your logs. Your device.", 11.0, muted()))
}
