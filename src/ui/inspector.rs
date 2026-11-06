use super::{controls::*, theme::*};
use crate::{
    app::State,
    log::{self, Event},
    query::{CorrelationField, InspectorTab, TimelineKind},
};
use rune::{El, ScrollSpec, Tx, col, row};

fn pair(title: &str, value: impl Into<String>) -> El {
    row()
        .gap(12.0)
        .py(6.0)
        .child(label(title, 11.0, muted()).w(85.0))
        .child(
            mono(value, 11.0, ink())
                .grow(1.0)
                .min_w(0.0)
                .wrap_anywhere(),
        )
}
fn fields(e: &Event) -> El {
    let mut out = col()
        .gap(5.0)
        .child(pair("Timestamp", e.field("timestamp")))
        .child(pair(
            "Source",
            format!(
                "{}{}",
                e.file(),
                e.line().map(|n| format!(":{n}")).unwrap_or_default()
            ),
        ))
        .child(pair("Message", e.field("message")))
        .child(pair("Method", e.field("method")))
        .child(pair("Path", e.field("path")))
        .child(pair(
            "Status",
            e.status()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
        ))
        .child(pair("Duration", log::duration_label(e.duration())));
    for field in [
        CorrelationField::Request,
        CorrelationField::Trace,
        CorrelationField::Session,
        CorrelationField::User,
    ] {
        let value = e.field(field.key());
        if value.is_empty() {
            continue;
        }
        let title = format!("{} ID", field);
        let v = value.clone();
        out = out.child(
            pair(&title, format!("{value}  ↗"))
                .key(format!("follow-{field}"))
                .role("button")
                .label(format!("Follow {field} {value}"))
                .focusable(true)
                .on_press(move |s: &mut State| s.follow(field, v.clone())),
        );
    }
    for (field, title, color) in [
        ("why", "WHY IT HAPPENED", warning_color()),
        ("fix", "SUGGESTED FIX", accent()),
    ] {
        if let Some(value) = e.value(field) {
            out = out.child(
                col()
                    .padding(12.0)
                    .gap(8.0)
                    .radius(6.0)
                    .fill(color.alpha(0.06))
                    .border(1.0, color.alpha(0.18))
                    .mt(8.0)
                    .child(label(title, 11.0, color))
                    .child(
                        label(
                            log::scalar(value).unwrap_or_else(|| value.to_string()),
                            12.0,
                            ink(),
                        )
                        .w_full()
                        .wrap_anywhere(),
                    ),
            );
        }
    }
    out
}
fn json_tree(s: &State, value: &serde_json::Value, prefix: &str) -> El {
    let key = format!("json-{prefix}");
    let entries: Vec<(String, &serde_json::Value)> = match value {
        serde_json::Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), v)).collect(),
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect(),
        _ => {
            return mono(value.to_string(), 11.0, accent())
                .key(key)
                .w_full()
                .wrap_anywhere();
        }
    };
    let mut out = col().key(key).gap(8.0);
    for (name, v) in entries {
        let path = format!("{prefix}/{}", name.replace('~', "~0").replace('/', "~1"));
        if v.is_object() || v.is_array() {
            let collapsed = s.collapsed.contains(&path);
            let toggle_path = path.clone();
            let count = v
                .as_object()
                .map(|m| m.len())
                .or_else(|| v.as_array().map(|a| a.len()))
                .unwrap_or(0);
            let mut group = col().gap(5.0).child(
                mono(
                    format!(
                        "{} {name}  {} {count} {}",
                        if collapsed { "▸" } else { "▾" },
                        if v.is_array() { "[" } else { "{" },
                        if v.is_array() { "]" } else { "}" }
                    ),
                    11.0,
                    muted(),
                )
                .key(format!("toggle-json-{path}"))
                .role("button")
                .label(format!("Toggle {name}"))
                .focusable(true)
                .w_full()
                .wrap_anywhere()
                .on_press(move |s: &mut State| {
                    if !s.collapsed.remove(&toggle_path) {
                        s.collapsed.insert(toggle_path.clone());
                    }
                }),
            );
            if !collapsed {
                group = group.child(json_tree(s, v, &path).pl(12.0));
            }
            out = out.child(group);
        } else {
            out = out.child(
                col()
                    .gap(3.0)
                    .child(
                        mono(format!("{name}:"), 11.0, json_key_color())
                            .w_full()
                            .wrap_anywhere(),
                    )
                    .child(
                        mono(
                            v.to_string(),
                            11.0,
                            if v.is_string() {
                                accent()
                            } else {
                                warning_color()
                            },
                        )
                        .w_full()
                        .wrap_anywhere()
                        .pl(10.0),
                    ),
            );
        }
    }
    out
}
fn timeline(s: &State, i: usize, kind: TimelineKind) -> El {
    let groups = s.workspace().timeline(i, kind);
    let mut out = col().gap(10.0).child(label(
        match kind {
            TimelineKind::Related => "Request / trace events · chronological",
            TimelineKind::Session => "Requests in this session · chronological",
        },
        11.0,
        muted(),
    ));
    for item in groups {
        let mut group = col().gap(5.0).padding(10.0).radius(6.0).border(1.0, line());
        if let Some(label) = item.label {
            group = group.child(mono(label, 11.0, accent()));
        }
        for j in item.events {
            let e = &s.workspace().events()[j];
            group = group.child(
                col()
                    .key(format!("timeline-{j}"))
                    .role("button")
                    .label(format!("Inspect {}", e.field("message")))
                    .focusable(true)
                    .gap(4.0)
                    .py(7.0)
                    .child(
                        row()
                            .gap(8.0)
                            .child(mono(e.time(s.utc), 11.0, muted()))
                            .child(label(
                                e.level().to_uppercase(),
                                11.0,
                                level_color(e.level()),
                            )),
                    )
                    .child(
                        label(e.field("message"), 11.0, ink())
                            .w_full()
                            .wrap_anywhere(),
                    )
                    .child(mono(e.field("path"), 11.0, accent()).w_full().truncate())
                    .on_press(move |s: &mut State| s.selected = Some(j)),
            );
        }
        out = out.child(group);
    }
    out
}
pub(super) fn inspector(s: &State, i: usize) -> El {
    let e = &s.workspace().events()[i];
    let mut tabs = row().gap(5.0);
    for tab in InspectorTab::ALL {
        tabs = tabs.child(
            button(tab.key(), tab.label(), s.tab == tab).on_press(move |s: &mut State| s.tab = tab),
        );
    }
    let content = match s.tab {
        InspectorTab::Json => json_tree(s, e.raw(), ""),
        InspectorTab::Timeline => timeline(s, i, TimelineKind::Related),
        InspectorTab::Session => timeline(s, i, TimelineKind::Session),
        InspectorTab::Fields => fields(e),
    };
    col()
        .key("inspector")
        .w(370.0)
        .min_h(0.0)
        .fill(panel())
        .border(1.0, line())
        .child(
            row()
                .h(50.0)
                .px(16.0)
                .center_y()
                .justify_between()
                .child(heading("Event inspector", 14.0))
                .child(button("copy-json", "Copy", false).on_press_tx(
                    move |s: &mut State, tx: &mut Tx| {
                        if let Some(event) = s.workspace().events().get(i) {
                            tx.copy(
                                serde_json::to_string_pretty(event.raw())
                                    .expect("JSON values are serializable"),
                            );
                            s.notice = "Event JSON copied".into();
                        }
                    },
                ))
                .child(
                    button("close-inspector", "×", false)
                        .on_press(|s: &mut State| s.selected = None)
                        .label("Close inspector"),
                ),
        )
        .child(
            col()
                .padding(16.0)
                .gap(12.0)
                .child(
                    row()
                        .gap(10.0)
                        .center_y()
                        .child(badge(
                            &e.level().to_uppercase(),
                            level_color(e.level()),
                            57.0,
                        ))
                        .child(mono(e.time(s.utc), 11.0, muted())),
                )
                .child(heading(e.field("message"), 16.0).w_full().wrap_anywhere())
                .child(tabs),
        )
        .child(rule())
        .child(
            col()
                .key("detail-scroll")
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(content.padding(16.0)),
        )
}
