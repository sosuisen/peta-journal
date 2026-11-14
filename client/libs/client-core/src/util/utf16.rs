//! Places in a text: in UTF-16 units as the text fields of the shells count, in characters
//! (Unicode scalars) as the core counts, and at the ends of letters as a person sees them
//! (ADR 043). Every shell converts with these.

use unicode_segmentation::UnicodeSegmentation;

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

/// The place `chars` characters into `text`, moved to the end of the letter it is inside, as a
/// person sees letters (a grapheme cluster). An emoji of a family is five characters and one
/// letter: a place the core finds itself never splits it (ADR 043).
pub fn after_letter(text: &str, chars: usize) -> usize {
    let mut counted = 0;
    for letter in text.graphemes(true) {
        if counted >= chars {
            break;
        }
        counted += letter.chars().count();
    }
    counted
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
