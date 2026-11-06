use super::theme::*;
use crate::app::State;
use rune::tok::TextStyle;
use rune::{Color, El, FileDialogOptions, TextEdit, Tx, el, row, text_input};

pub(super) fn button(key: impl Into<String>, title: impl Into<String>, active: bool) -> El {
    let title = title.into();
    row()
        .key(key)
        .role("button")
        .label(&title)
        .focusable(true)
        .h(32.0)
        .px(11.0)
        .center_y()
        .radius(6.0)
        .fill(if active { active_surface() } else { panel() })
        .border(1.0, if active { active_border() } else { line() })
        .hover(Color::WHITE.alpha(0.045))
        .child(label(title, 13.0, if active { accent() } else { ink() }))
}
pub(super) fn import_button() -> El {
    button("import", "+ Import files", true).on_press_tx(|_: &mut State, tx: &mut Tx| {
        tx.open_file(
            FileDialogOptions {
                title: "Import structured logs".into(),
                ..Default::default()
            }
            .filter("Log files", &["json", "jsonl", "ndjson"]),
            |s: &mut State, result, _| match result {
                Ok(Some(path)) => s.import(vec![path]),
                Ok(None) => {}
                Err(e) => {
                    s.notice = format!(
                        "Could not open file chooser: {e}. You can drop files into the window."
                    )
                }
            },
        );
    })
}
pub(super) fn export_button() -> El {
    button("export", "Export NDJSON", false).on_press_tx(|_: &mut State, tx: &mut Tx| {
        tx.save_file(
            FileDialogOptions {
                title: "Export matching events".into(),
                file_name: Some("events.ndjson".into()),
                ..Default::default()
            }
            .filter("NDJSON", &["ndjson"]),
            |s: &mut State, result, _| match result {
                Ok(Some(path)) => s.export(path),
                Ok(None) => {}
                Err(e) => s.notice = format!("Could not export: {e}"),
            },
        );
    })
}
pub(super) fn input(
    key: &str,
    edit: &TextEdit,
    placeholder: &str,
    lens: fn(&mut State) -> &mut TextEdit,
) -> El {
    el().h(34.0)
        .px(10.0)
        .center_y()
        .radius(6.0)
        .border(1.0, line())
        .fill(bg())
        .child(
            text_input(edit, TextStyle::new(13.0, 400.0))
                .key(key)
                .label(placeholder)
                .w_full()
                .single_line()
                .color(ink())
                .placeholder(placeholder, muted())
                .on_edit(lens),
        )
}
pub(super) fn nav(
    key: impl Into<String>,
    title: impl Into<String>,
    count: impl Into<String>,
    active: bool,
    f: impl Fn(&mut State) + 'static,
) -> El {
    let title = title.into();
    row()
        .key(key)
        .role("button")
        .label(&title)
        .focusable(true)
        .w_full()
        .h(34.0)
        .px(10.0)
        .center_y()
        .justify_between()
        .radius(5.0)
        .fill(if active {
            active_surface()
        } else {
            Color::TRANSPARENT
        })
        .hover(Color::WHITE.alpha(0.04))
        .child(
            label(title, 13.0, if active { accent() } else { muted() })
                .grow(1.0)
                .min_w(0.0)
                .truncate(),
        )
        .child(mono(count, 11.0, muted()))
        .on_press(f)
}
pub(super) fn badge(value: &str, color: Color, width: f32) -> El {
    el().w(width)
        .h(22.0)
        .radius(4.0)
        .fill(color.alpha(0.10))
        .center()
        .child(mono(value, 11.0, color))
}
pub(super) fn level_color(level: &str) -> Color {
    match level {
        "error" | "fatal" => error_color(),
        "warn" => warning_color(),
        "info" => info_color(),
        _ => muted(),
    }
}
