//! The width of a card as `atlas.js` draws it from its label, measured in the font of the labels.
//! A program that hides a label from the scene, such as a static site that hides its drafts,
//! measures the label here and gives the card that width, so the card keeps its size.

#[cfg(test)]
mod tests {
    use super::*;

    /// The font PetaJournal draws the labels in
    const FONT: &[u8] = include_bytes!("../../frontend-rust/assets/NotoSansJP-Regular.ttf");

    fn font() -> LabelFont {
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

    // 長いラベルは折り返すので、カードは折り返す幅より広くならない
    #[test]
    fn a_long_label_wraps_at_the_widest_card() {
        let long = "あ".repeat(40);
        assert_eq!(font().card_width(&long), (320.0_f64 - 24.0 + 3.0 + 24.0).ceil());
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
