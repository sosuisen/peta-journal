//! A place in a text, counted in UTF-16 units as the text fields of the shells count, and in
//! characters (Unicode scalars) as the core counts (ADR 043). Every shell converts with these.

/// The characters before the place `units` UTF-16 units into `text`. A place inside a
/// surrogate pair goes after its character; a place past the end is the end.
pub fn chars_of_utf16(text: &str, units: usize) -> usize {
    let mut counted = 0;
    text.chars()
        .take_while(|c| {
            let before = counted;
            counted += c.len_utf16();
            before < units
        })
        .count()
}

/// The UTF-16 units before the place `chars` characters into `text`. A place past the end is
/// the end.
pub fn utf16_of_chars(text: &str, chars: usize) -> usize {
    text.chars().take(chars).map(char::len_utf16).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    // シェルのUTF-16の位置は、コアの文字の数になる。絵文字はUTF-16では2つだが、1文字である。
    #[test]
    fn a_place_in_utf16_units_is_counted_in_characters() {
        assert_eq!(chars_of_utf16("日本😀語", 0), 0);
        assert_eq!(chars_of_utf16("日本😀語", 2), 2);
        assert_eq!(chars_of_utf16("日本😀語", 4), 3);
        assert_eq!(chars_of_utf16("日本😀語", 5), 4);
    }

    // サロゲートペアの途中の位置は、その文字の後ろになる。本文より先の位置は、末尾になる。
    #[test]
    fn a_place_inside_a_surrogate_pair_goes_after_the_character() {
        assert_eq!(chars_of_utf16("日本😀語", 3), 3);
        assert_eq!(chars_of_utf16("日本😀語", 99), 4);
    }

    // コアの文字の数は、UTF-16の位置に戻せる。本文より先の位置は、末尾になる。
    #[test]
    fn a_place_in_characters_is_counted_in_utf16_units() {
        assert_eq!(utf16_of_chars("日本😀語", 0), 0);
        assert_eq!(utf16_of_chars("日本😀語", 3), 4);
        assert_eq!(utf16_of_chars("日本😀語", 4), 5);
        assert_eq!(utf16_of_chars("日本😀語", usize::MAX), 5);
    }
}
