//! The plugin's colour palette, without egui, so code outside the editor
//! (and the CI lane, built without the `gui` feature) can read it. The
//! editor builds its `Color32` constants from these values, and the site's
//! design tokens (`web/theme/tokens.css`) must match them
//! (`add-site-shell`).

/// Main background.
pub const CHARCOAL: [u8; 3] = [0x0D, 0x0D, 0x0D];
/// Panel background.
pub const DARK_STEEL: [u8; 3] = [0x1A, 0x1A, 0x1A];
/// Panel borders and separators.
pub const CONCRETE: [u8; 3] = [0x33, 0x33, 0x33];
/// Accent: active values, titles.
pub const SAFETY_YELLOW: [u8; 3] = [0xFA, 0xCC, 0x15];
/// Secondary accent.
pub const RUST_ORANGE: [u8; 3] = [0xC2, 0x41, 0x0C];
/// Warnings and the diode meter.
pub const DANGER_RED: [u8; 3] = [0xB9, 0x1C, 0x1C];
/// Main text.
pub const OFF_WHITE: [u8; 3] = [0xD4, 0xD4, 0xD4];
/// Secondary text and help badges.
pub const MUTED_GREY: [u8; 3] = [0x73, 0x73, 0x73];
/// Light steel: inactive outlines.
pub const STEEL_LIGHT: [u8; 3] = [0x55, 0x55, 0x55];
/// Recessed fields.
pub const RECESSED_BG: [u8; 3] = [0x11, 0x11, 0x11];
/// Fader tracks.
pub const FADER_TRACK_BG: [u8; 3] = [0x00, 0x00, 0x00];
/// Fader thumbs.
pub const THUMB_BG: [u8; 3] = [0x44, 0x44, 0x44];
/// Fader thumb borders.
pub const THUMB_BORDER: [u8; 3] = [0x66, 0x66, 0x66];

/// Every palette colour with its name, in declaration order.
pub const PALETTE: [(&str, [u8; 3]); 13] = [
    ("CHARCOAL", CHARCOAL),
    ("DARK_STEEL", DARK_STEEL),
    ("CONCRETE", CONCRETE),
    ("SAFETY_YELLOW", SAFETY_YELLOW),
    ("RUST_ORANGE", RUST_ORANGE),
    ("DANGER_RED", DANGER_RED),
    ("OFF_WHITE", OFF_WHITE),
    ("MUTED_GREY", MUTED_GREY),
    ("STEEL_LIGHT", STEEL_LIGHT),
    ("RECESSED_BG", RECESSED_BG),
    ("FADER_TRACK_BG", FADER_TRACK_BG),
    ("THUMB_BG", THUMB_BG),
    ("THUMB_BORDER", THUMB_BORDER),
];

#[cfg(test)]
mod tests {
    use super::PALETTE;
    use std::collections::BTreeMap;

    /// `--charcoal: #0D0D0D;` lines of the site's design tokens, by
    /// constant name (`--dark-steel` → `DARK_STEEL`). Aliases such as
    /// `--bg: var(--charcoal)` aren't colours and are skipped.
    fn token_colours(css: &str) -> BTreeMap<String, String> {
        css.lines()
            .filter_map(|line| {
                let line = line.trim();
                let rest = line.strip_prefix("--")?;
                let (name, value) = rest.split_once(':')?;
                let value = value.trim().trim_end_matches(';').trim();
                let hex = value.strip_prefix('#')?;
                (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| {
                    (
                        name.trim().replace('-', "_").to_ascii_uppercase(),
                        hex.to_ascii_uppercase(),
                    )
                })
            })
            .collect()
    }

    /// The site's tokens and the plugin palette hold the same colours
    /// (`site-shell`: shared dark theme). Fails naming each colour that is
    /// missing on either side or differs.
    #[test]
    fn site_theme_matches_editor_palette() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/web/theme/tokens.css");
        let css =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("can't read {path}: {e}"));
        let tokens = token_colours(&css);
        let mut problems = Vec::new();
        for (name, [r, g, b]) in PALETTE {
            let want = format!("{r:02X}{g:02X}{b:02X}");
            match tokens.get(name) {
                None => problems.push(format!("{name} (#{want}) is missing from tokens.css")),
                Some(got) if *got != want => {
                    problems.push(format!("{name}: palette #{want}, tokens.css #{got}"))
                }
                Some(_) => {}
            }
        }
        for name in tokens.keys() {
            if !PALETTE.iter().any(|(n, _)| n == name) {
                problems.push(format!("tokens.css colour {name} is not in the palette"));
            }
        }
        assert!(
            problems.is_empty(),
            "site theme drifted from the palette:\n{}",
            problems.join("\n")
        );
    }
}
