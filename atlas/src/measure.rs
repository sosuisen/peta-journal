//! The width of a card as `atlas.js` draws it from its label, measured in the font of the labels.
//! A program that hides a label from the scene, such as a static site that hides its drafts,
//! measures the label here and gives the card that width, so the card keeps its size.

use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{Face, UnicodeBuffer};

// The style of the labels in `card()` of `atlas.js`. A change there must come here too.
/// The size of the characters of a label, in pixels of the canvas
const FONT_SIZE: f64 = 13.0;
/// The width of the white edge around the characters. PixiJS counts it in the width of a text.
const STROKE_WIDTH: f64 = 3.0;
/// The space between the label of a card and its sides
const CARD_PADDING: f64 = 12.0;
/// The narrowest and the widest a card grows to fit its label
const CARD_MIN_WIDTH: f64 = 40.0;
const CARD_MAX_WIDTH: f64 = 320.0;

/// The font of the labels of the cards.
pub struct LabelFont<'a> {
    face: Face<'a>,
}

impl<'a> LabelFont<'a> {
    /// The font in the bytes of a font file, or none when they hold no font.
    pub fn from_bytes(data: &'a [u8]) -> Option<LabelFont<'a>> {
        Face::from_slice(data, 0).map(|face| LabelFont { face })
    }

    /// The width of a card at scale 1 whose width is left to its label, as `atlas.js` draws it:
    /// the widest line of the label as PixiJS wraps it, with the white edge and the padding.
    pub fn card_width(&self, label: &str) -> f64 {
        let text_width = self
            .lines(label)
            .iter()
            .map(|line| self.line_width(line))
            .fold(0.0, f64::max)
            + STROKE_WIDTH;
        (text_width + 2.0 * CARD_PADDING).max(CARD_MIN_WIDTH).ceil()
    }

    /// The lines of a label as PixiJS 8 wraps a text with `wordWrap` and `breakWords`. The
    /// label is cut into words after each breaking space and hyphen. A word goes to the next
    /// line when it does not fit; a word wider than a whole line is cut after each character.
    /// A space does not start a line, and the spaces at the end of a line go.
    fn lines(&self, label: &str) -> Vec<String> {
        let wrap_width = CARD_MAX_WIDTH - 2.0 * CARD_PADDING;
        let mut lines = Vec::new();
        let mut line = String::new();
        let mut width = 0.0;
        // Spaces are kept only at the start of the text, before any line has been cut
        let mut first_line = true;
        for word in words(label) {
            let word_width = self.advance(&word);
            if word_width > wrap_width {
                if !line.is_empty() {
                    lines.push(trim_spaces(&line));
                    line.clear();
                    width = 0.0;
                }
                for c in word.chars() {
                    let c_width = self.advance(c.encode_utf8(&mut [0; 4]));
                    if c_width + width > wrap_width {
                        lines.push(trim_spaces(&line));
                        first_line = false;
                        line.clear();
                        width = 0.0;
                    }
                    line.push(c);
                    width += c_width;
                }
                continue;
            }
            if word_width + width > wrap_width {
                first_line = false;
                lines.push(trim_spaces(&line));
                line.clear();
                width = 0.0;
            }
            if !line.is_empty() || !is_breaking_space(&word) || first_line {
                line.push_str(&word);
                width += word_width;
            }
        }
        let last = trim_spaces(&line);
        if !last.is_empty() {
            lines.push(last);
        }
        lines
    }

    /// The width of a line as PixiJS measures it: how far the line reaches, or the width of its
    /// ink when the ink reaches further, as the ink of an X does past its advance. Chromium gives
    /// the ink of each glyph in whole pixels from where the glyph starts: one pixel more at the
    /// left, and rounded up at the right.
    fn line_width(&self, line: &str) -> f64 {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(line);
        let glyphs = rustybuzz::shape(&self.face, &[], buffer);
        let mut pen = 0.0;
        let (mut left, mut right) = (f64::INFINITY, f64::NEG_INFINITY);
        for (info, pos) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
            let glyph = GlyphId(info.glyph_id as u16);
            if let Some(ink) = self.face.glyph_bounding_box(glyph) {
                let x = pen + self.pixels(i64::from(pos.x_offset));
                left = left.min(x + self.pixels(i64::from(ink.x_min)).floor() - 1.0);
                right = right.max(x + self.pixels(i64::from(ink.x_max)).ceil());
            }
            pen += self.pixels(i64::from(pos.x_advance));
        }
        if left <= right {
            pen.max(right - left)
        } else {
            pen
        }
    }

    /// How far a text reaches in pixels, shaped as a browser shapes it, with kerning
    fn advance(&self, text: &str) -> f64 {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        let glyphs = rustybuzz::shape(&self.face, &[], buffer);
        let units: i64 = glyphs
            .glyph_positions()
            .iter()
            .map(|p| i64::from(p.x_advance))
            .sum();
        self.pixels(units)
    }

    fn pixels(&self, units: i64) -> f64 {
        units as f64 * FONT_SIZE / f64::from(self.face.units_per_em())
    }
}

/// The spaces that PixiJS breaks a line at
fn is_breaking_space(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|c| matches!(c as u32, 9 | 32 | 8192..=8198 | 8200..=8202 | 8287 | 12288))
}

/// The hyphens that PixiJS breaks a line after
fn is_hyphen(c: char) -> bool {
    matches!(c as u32, 45 | 8208 | 8211 | 8212 | 173)
}

/// The words of a text as PixiJS cuts them: each breaking space alone, and a word ends after
/// a hyphen that a letter follows.
fn words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if is_breaking_space(c.encode_utf8(&mut [0; 4])) {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            words.push(c.to_string());
            continue;
        }
        word.push(c);
        let next_is_letter = chars
            .get(i + 1)
            .is_some_and(|n| !is_breaking_space(n.encode_utf8(&mut [0; 4])));
        if is_hyphen(c) && next_is_letter {
            words.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// A line without the breaking spaces at its end
fn trim_spaces(line: &str) -> String {
    line.trim_end_matches(|c: char| is_breaking_space(c.encode_utf8(&mut [0; 4])))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The font PetaJournal draws the labels in
    const FONT: &[u8] = include_bytes!("../../frontend-rust/assets/NotoSansJP-Regular.ttf");

    fn font() -> LabelFont<'static> {
        LabelFont::from_bytes(FONT).unwrap()
    }

    // 全角の文字は1文字が13pxで、白い縁の3pxと左右の余白の12pxずつを足した幅になる
    #[test]
    fn a_card_is_as_wide_as_its_label_with_its_edge_and_padding() {
        assert_eq!(font().card_width("支持体として"), 6.0 * 13.0 + 3.0 + 24.0);
    }

    // ラベルが短くても、カードは40pxより狭くならない
    #[test]
    fn a_card_is_no_narrower_than_its_least_width() {
        assert_eq!(font().card_width(""), 40.0);
        assert_eq!(font().card_width("a"), 40.0);
    }

    // 長いラベルは、入るところまでの文字で折り返し、いちばん広い行の幅になる
    #[test]
    fn a_long_label_wraps_after_the_last_character_that_fits() {
        // 22 characters of 13px fit in the 296px of a line
        assert_eq!(
            font().card_width(&"あ".repeat(40)),
            22.0 * 13.0 + 3.0 + 24.0
        );
    }

    // PixiJS 8が、Chromiumでこのフォントで描いたカードの幅と同じになる
    #[test]
    fn a_card_is_as_wide_as_pixijs_draws_it_in_chromium() {
        // Read back from atlas.js in headless Chromium, at a device pixel ratio of 1
        let drawn = [
            ("支持体としてのビデオメモリー", 209.0),
            ("Bike OutlinerとLogseq OG", 184.0),
            // The ink of the X reaches past its advance
            ("VSCodeで始めるJavaFX", 170.0),
            ("sss-wallpaper 制作ノート", 179.0),
            ("JavaFX Development on VSCode", 223.0),
            ("Picturesque Marks", 139.0),
            ("ノートとADR", 105.0),
            // The ink of the A at the start reaches a pixel to the left of the line
            ("AVA Wave Type", 119.0),
            (
                "アウトラインを記録するためのMarkdown解釈という長い題名のページ",
                312.0,
            ),
        ];
        for (label, width) in drawn {
            assert_eq!(font().card_width(label), width, "{label}");
        }
    }

    // 欧文の幅は文字ごとに違い、カーニングも入る
    #[test]
    fn latin_letters_take_their_own_widths() {
        let narrow = font().card_width("iiiiiiiiii");
        let wide = font().card_width("WWWWWWWWWW");
        assert!(narrow < wide, "{narrow} {wide}");
    }

    // フォントでないバイト列からは、フォントを作れない
    #[test]
    fn bytes_that_are_no_font_make_no_font() {
        assert!(LabelFont::from_bytes(b"not a font").is_none());
    }
}
