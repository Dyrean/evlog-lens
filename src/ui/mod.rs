mod branding;
pub(crate) use branding::export_logo;
mod controls;
mod explorer;
mod inspector;
mod sidebar;
mod theme;
use self::{controls::*, theme::*};
use crate::app::State;
use rune::{Cx, El, ScrollSpec, col, el, row};

pub(crate) fn view(s: &State, _: &mut Cx<State>) -> El {
    let filtered = s.results();
    let ids = filtered
        .as_ref()
        .map(|result| result.ids.as_slice())
        .unwrap_or_default();
    let mut content = col().grow(1.0).min_w(0.0).min_h(0.0).child(
        row()
            .h(70.0)
            .px(22.0)
            .center_y()
            .justify_between()
            .border(0.5, line())
            .child(
                col()
                    .gap(5.0)
                    .child(heading("Event explorer", 20.0))
                    .child(label("Follow the signal. Find the story.", 11.0, muted())),
            )
            .child(
                row()
                    .gap(8.0)
                    .child(if s.workspace().events().is_empty() {
                        el()
                    } else {
                        button("save-session", "Save session", false).on_press(State::save_session)
                    })
                    .child(if s.workspace().events().is_empty() {
                        el()
                    } else {
                        button("clear", "Clear logs", false).on_press(State::clear)
                    })
                    .child(import_button()),
            ),
    );
    if s.importing() {
        content = content.child(
            row()
                .padding(12.0)
                .gap(15.0)
                .center_y()
                .fill(active_surface())
                .child(
                    label(format!("Importing {}", s.import_progress()), 12.0, accent())
                        .grow(1.0)
                        .truncate(),
                )
                .child(button("cancel-import", "Cancel import", false).on_press(
                    |s: &mut State| {
                        s.cancel_import();
                        s.notice = "Import cancelled; loaded events retained".into();
                    },
                )),
        );
    }
    if s.workspace().events().is_empty() {
        content = content.child(explorer::welcome(s));
    } else {
        content = content.child(
            col()
                .key("filter-scroll")
                .min_h(0.0)
                .shrink()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    col()
                        .child(explorer::overview(s, ids))
                        .child(explorer::toolbar(s, filtered.as_ref().err())),
                ),
        );
        let mut body = row().grow(1.0).min_h(180.0).child(explorer::table(s, ids));
        if let Some(i) = s.selected.filter(|i| *i < s.workspace().events().len()) {
            body = body.child(inspector::inspector(s, i));
        }
        content = content.child(body);
    }
    if !s.workspace().errors().is_empty() {
        content = content.child(
            button(
                "import-issues",
                format!(
                    "{} import issues · {}",
                    s.workspace().errors().len(),
                    if s.show_errors {
                        "Hide"
                    } else {
                        "Show details"
                    }
                ),
                false,
            )
            .on_press(|s: &mut State| s.show_errors = !s.show_errors)
            .mx(18.0),
        );
        if s.show_errors {
            content = content.child(
                col()
                    .h(100.0)
                    .scroll(ScrollSpec {
                        stick_bottom: false,
                    })
                    .child(
                        col()
                            .padding(14.0)
                            .children(s.workspace().errors().iter().map(|e| {
                                label(
                                    format!(
                                        "{}:{}  {}",
                                        e.file,
                                        e.line.map(|n| n.to_string()).unwrap_or_default(),
                                        e.message
                                    ),
                                    11.0,
                                    warning_color(),
                                )
                                .w_full()
                                .wrap_anywhere()
                            })),
                    ),
            );
        }
    }
    if !s.notice.is_empty() {
        content = content.child(
            row()
                .px(18.0)
                .h(32.0)
                .gap(8.0)
                .center_y()
                .child(
                    label(&s.notice, 11.0, accent())
                        .key("notice")
                        .grow(1.0)
                        .truncate(),
                )
                .child(
                    button("dismiss-notice", "×", false).on_press(|s: &mut State| s.notice.clear()),
                ),
        );
    }
    content = content.child(
        row()
            .h(30.0)
            .px(18.0)
            .center_y()
            .justify_between()
            .border(0.5, line())
            .child(label("●  Processed on your device", 11.0, muted()))
            .child(if s.has_saved_session() {
                button("delete-session", "Delete saved session", false)
                    .on_press(State::delete_session)
            } else {
                label("Sessions are saved only when you choose", 11.0, muted())
            }),
    );
    row()
        .key("app")
        .w_full()
        .h_full()
        .fill(bg())
        .child(sidebar::sidebar(s))
        .child(content)
        .on_drop_files(|s: &mut State, files, _| s.import(files.to_vec()))
        .on_drop_hover(|s: &mut State, over, _| s.drop_over = over)
}
