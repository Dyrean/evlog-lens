use rune::el;
use rune::tok::TextStyle;
use rune::{Color, El, text};

pub(super) fn c(hex: &str) -> Color {
    rune::color::hex(hex).expect("valid palette color")
}
pub(super) fn bg() -> Color {
    c("#0c1220")
}
pub(super) fn panel() -> Color {
    c("#141d2e")
}
pub(super) fn line() -> Color {
    c("#29354b")
}
pub(super) fn ink() -> Color {
    c("#edf2ff")
}
pub(super) fn muted() -> Color {
    c("#a5b1c8")
}
pub(super) const ACCENT_HEX: &str = "#b5afff";
pub(super) const MINT_HEX: &str = "#71e1d0";
pub(super) const LOGO_TILE_HEX: &str = "#202541";
pub(super) fn accent() -> Color {
    c(ACCENT_HEX)
}
pub(super) fn label(value: impl Into<String>, size: f32, color: Color) -> El {
    text(value).font(TextStyle::new(size, 400.0)).color(color)
}
pub(super) fn mono(value: impl Into<String>, size: f32, color: Color) -> El {
    text(value)
        .font(TextStyle::new(size, 400.0).mono())
        .color(color)
}
pub(super) fn heading(value: impl Into<String>, size: f32) -> El {
    text(value)
        .font(TextStyle {
            tracking: -0.2,
            ..TextStyle::new(size, 600.0)
        })
        .color(ink())
}
pub(super) fn rule() -> El {
    el().w_full().h(1.0).fill(line())
}

pub(super) fn active_surface() -> Color {
    c("#292b4d")
}

pub(super) fn active_border() -> Color {
    c("#5d568b")
}

pub(super) fn sidebar_surface() -> Color {
    c("#101827")
}

pub(super) fn selected_surface() -> Color {
    c("#252c4a")
}

pub(super) fn error_color() -> Color {
    c("#ff9aab")
}

pub(super) fn warning_color() -> Color {
    c("#f2cc8f")
}

pub(super) fn info_color() -> Color {
    c("#9fc7ff")
}

pub(super) fn json_key_color() -> Color {
    c("#a3bfff")
}

pub(super) fn mint() -> Color {
    c(MINT_HEX)
}
