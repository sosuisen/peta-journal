//! Where a touch on the shown text of an item puts the cursor in its Markdown. The AST does not
//! say where in the Markdown its nodes stand, so the letters are lined up instead.

/// What leaving out a letter of the shown text costs, where a letter of the Markdown costs 1.
/// The Markdown has many letters that are not shown, such as the address of a link; the shown
/// text has few of its own, such as the title of an alert. A link longer than this, with only
/// a letter or two after it, puts the cursor before its address.
const SHOWN_LEFT_OUT: usize = 32;

/// The place in `markdown`, in characters, that stands right after `shown_before`: the letters
/// of the shown text before a touch.
///
/// The shown text is the Markdown without its marks, such as `**` and the address of a link,
/// and with a few letters of its own, such as the title of an alert. The two are lined up so
/// that the fewest letters are left out, and the place is the first one where that few are.
/// A letter of the shown text is lined up whenever the Markdown has it within
/// [`SHOWN_LEFT_OUT`] letters, and left out when it is further on: then only the shown text
/// has it.
pub fn markdown_place(markdown: &str, shown_before: &str) -> usize {
    let markdown: Vec<char> = markdown.chars().collect();
    // What leaving out letters costs at least, to line up the letters so far with the first
    // `j` of the Markdown. Only one row is kept: a body may be long
    let mut row: Vec<usize> = (0..=markdown.len()).collect();
    for letter in shown_before.chars() {
        let mut next = Vec::with_capacity(row.len());
        next.push(row[0] + SHOWN_LEFT_OUT);
        for (j, &mark) in markdown.iter().enumerate() {
            let left_out = (next[j] + 1).min(row[j + 1] + SHOWN_LEFT_OUT);
            next.push(if mark == letter {
                left_out.min(row[j])
            } else {
                left_out
            });
        }
        row = next;
    }
    let least = row.iter().min().copied().unwrap_or(0);
    row.iter().position(|&cost| cost == least).unwrap_or(0)
}
