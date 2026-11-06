use super::theme::{ACCENT_HEX, LOGO_TILE_HEX, MINT_HEX, c};
use rune::{El, LineCap, Shape, canvas};
use std::path::Path;

const SIZE: f32 = 64.0;
const CORNER: f32 = 16.0;
const CENTER: [f32; 2] = [28.0, 28.0];
const RADIUS: f32 = 15.0;
const RING_WIDTH: f32 = 3.0;
const LINES: [([f32; 2], [f32; 2], f32, &str); 4] = [
    ([39.0, 39.0], [49.0, 49.0], 5.0, ACCENT_HEX),
    ([21.0, 23.0], [34.0, 23.0], 2.5, MINT_HEX),
    ([21.0, 29.0], [30.0, 29.0], 2.5, MINT_HEX),
    ([21.0, 35.0], [26.0, 35.0], 2.5, MINT_HEX),
];

pub(super) fn logo(size: f32) -> El {
    let ring: Vec<_> = (0..=64)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 64.0;
            [
                CENTER[0] + RADIUS * angle.cos(),
                CENTER[1] + RADIUS * angle.sin(),
            ]
        })
        .collect();
    let mut shape = Shape::new().stroke(ring, RING_WIDTH, c(ACCENT_HEX));
    for (start, end, width, color) in LINES {
        shape = shape.line(start, end, width, LineCap::Round, c(color));
    }
    canvas(shape.view_box(SIZE, SIZE))
        .key("lens-logo")
        .size(size, size)
        .radius(size * CORNER / SIZE)
        .fill(c(LOGO_TILE_HEX))
        .role("img")
        .label("Evlog Lens logo")
}
fn svg() -> String {
    use std::fmt::Write;
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 {SIZE} {SIZE}" role="img" aria-labelledby="title desc">
  <title id="title">Evlog Lens</title>
  <desc id="desc">A lavender magnifying lens containing three mint event lines on a midnight blue tile.</desc>
  <rect width="{SIZE}" height="{SIZE}" rx="{CORNER}" fill="{LOGO_TILE_HEX}"/>
  <circle cx="{}" cy="{}" r="{RADIUS}" fill="none" stroke="{ACCENT_HEX}" stroke-width="{RING_WIDTH}"/>
"#,
        CENTER[0], CENTER[1]
    );
    for (start, end, width, color) in LINES {
        writeln!(svg,r#"  <path d="M{} {} {} {}" stroke="{color}" stroke-width="{width}" stroke-linecap="round"/>"#,start[0],start[1],end[0],end[1]).expect("formatting a String cannot fail");
    }
    svg.push_str("</svg>\n");
    svg
}
pub(crate) fn export_logo(path: &Path) -> Result<(), String> {
    std::fs::write(path, svg()).map_err(|e| format!("Could not export logo: {e}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn checked_in_svg_matches_the_native_logo_geometry() {
        assert_eq!(
            super::svg(),
            include_str!("../../assets/evlog-lens.svg"),
            "Regenerate with --export-logo assets/evlog-lens.svg"
        );
    }
}
