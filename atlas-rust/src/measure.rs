//! The width of a card as `atlas.js` draws it from its label, measured in the font of the labels.
//! A program that hides a label from the scene, such as a static site that hides its drafts,
//! measures the label here and gives the card that width, so the card keeps its size.

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

    /// The width of a card at scale 1 whose width is left to its label, as `atlas.js` draws it.
    /// A label wider than the widest card wraps, so its widest line is taken as that width.
    /// The lines of a wrapped label break between words, so the true line may be a little
    /// narrower; a hidden label has no lines to show, and the card only keeps its size.
    pub fn card_width(&self, label: &str) -> f64 {
        let wrap_width = CARD_MAX_WIDTH - 2.0 * CARD_PADDING;
        let text_width = self.advance(label).min(wrap_width) + STROKE_WIDTH;
        (text_width + 2.0 * CARD_PADDING).max(CARD_MIN_WIDTH).ceil()
    }

    /// How far the label reaches in pixels, shaped as a browser shapes it, with kerning
    fn advance(&self, label: &str) -> f64 {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(label);
        let glyphs = rustybuzz::shape(&self.face, &[], buffer);
        let units: i64 = glyphs
            .glyph_positions()
            .iter()
            .map(|p| i64::from(p.x_advance))
            .sum();
        units as f64 * FONT_SIZE / f64::from(self.face.units_per_em())
    }
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
            // The words are measured one by one, so no kerning crosses a space
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
