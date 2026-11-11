//! Fractional indexes for the order of the items of a page: strings that sort in byte order,
//! with room for one between any two. The scheme is rocicorp's fractional-indexing in base
//! 62. An index is an integer part, whose first character tells its length (`a` is two
//! characters, `b` three, ... and `Z` two, `Y` three, ... going down), and then a fraction
//! with no trailing `0`. Appending counts the integer part up, so the indexes stay short;
//! inserting into one gap over and over grows the fraction by a character every six inserts
//! or so. Both are kept short enough by the reindexing the callers do.
//!
//! The indexes of the scheme before this one (`z0`, `y`, `zzV`...) are not valid here. Making
//! an index next to one fails with `NoGap`, and the caller reindexes the page.

const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const RADIX: usize = DIGITS.len();
const FIRST: &str = "a0";
/// The longest an index may grow before the items around it are reindexed.
pub const MAX_LEN: usize = 16;
/// An index short enough to stay as it is and bound a reindex.
const SHORT_LEN: usize = 8;

/// No index fits between the two given, or one of them is not an index of this scheme.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct NoGap(pub String);

/// The index of the first item of a page.
pub fn initial() -> String {
    FIRST.to_string()
}

/// An index between the two, after the first alone, before the second alone, or the initial
/// one when there is neither.
pub fn between(before: Option<&str>, after: Option<&str>) -> Result<String, NoGap> {
    if let Some(a) = before {
        validate(a)?;
    }
    if let Some(b) = after {
        validate(b)?;
    }
    match (before, after) {
        (None, None) => Ok(FIRST.to_string()),
        (None, Some(b)) => {
            let (ib, fb) = split(b);
            if !fb.is_empty() {
                return Ok(ib.to_string());
            }
            decrement_integer(ib).ok_or_else(|| NoGap(format!("No index before {b}")))
        }
        (Some(a), None) => {
            let (ia, fa) = split(a);
            Ok(match increment_integer(ia) {
                Some(next) => next,
                None => format!("{ia}{}", midpoint(fa, None)),
            })
        }
        (Some(a), Some(b)) => {
            if a >= b {
                return Err(NoGap(format!("No gap between {a} and {b}")));
            }
            let (ia, fa) = split(a);
            let (ib, fb) = split(b);
            if ia == ib {
                return Ok(format!("{ia}{}", midpoint(fa, Some(fb))));
            }
            Ok(match increment_integer(ia) {
                Some(next) if next.as_str() < b => next,
                _ => format!("{ia}{}", midpoint(fa, None)),
            })
        }
    }
}

/// `count` indexes in order between the two: spread over the gap when both are given, else
/// counted on from the one given.
pub fn between_many(
    before: Option<&str>,
    after: Option<&str>,
    count: usize,
) -> Result<Vec<String>, NoGap> {
    let mut out = Vec::with_capacity(count);
    if let (Some(b), Some(a)) = (before, after) {
        fill_between(b, a, count, &mut out)?;
        return Ok(out);
    }
    let mut current = after.or(before).map(str::to_string);
    for _ in 0..count {
        let next = match after {
            None => between(current.as_deref(), None)?,
            Some(_) => between(None, current.as_deref())?,
        };
        out.push(next.clone());
        current = Some(next);
    }
    if after.is_some() {
        out.reverse();
    }
    Ok(out)
}

fn fill_between(
    before: &str,
    after: &str,
    count: usize,
    out: &mut Vec<String>,
) -> Result<(), NoGap> {
    if count == 0 {
        return Ok(());
    }
    let mid = between(Some(before), Some(after))?;
    let left = count / 2;
    fill_between(before, &mid, left, out)?;
    out.push(mid.clone());
    fill_between(&mid, after, count - 1 - left, out)
}

/// `count` indexes for a page arranged from the start.
pub fn generate_indices(count: usize) -> Vec<String> {
    let mut indices = Vec::with_capacity(count);
    let mut previous: Option<String> = None;
    for _ in 0..count {
        let next = between(previous.as_deref(), None)
            .expect("counting the integer part up from the first index never fails");
        indices.push(next.clone());
        previous = Some(next);
    }
    indices
}

/// Fresh indexes for the items around every index longer than `MAX_LEN`: the run from the
/// nearest short index before it to the nearest short index after it is spread over the gap
/// between those two, which keep theirs. Returns the positions that change, with their new
/// index. The rest of the page is untouched, so another device editing it meets as few
/// changed indexes as possible.
pub fn shorten(indices: &[&str]) -> Result<Vec<(usize, String)>, NoGap> {
    let mut changes = Vec::new();
    let mut i = 0;
    while i < indices.len() {
        if indices[i].len() <= MAX_LEN {
            i += 1;
            continue;
        }
        let left = (0..i).rev().find(|&k| indices[k].len() <= SHORT_LEN);
        let right = (i..indices.len()).find(|&k| indices[k].len() <= SHORT_LEN);
        let start = left.map_or(0, |k| k + 1);
        let end = right.unwrap_or(indices.len());
        let fresh = between_many(
            left.map(|k| indices[k]),
            right.map(|k| indices[k]),
            end - start,
        )?;
        changes.extend((start..end).zip(fresh));
        i = end;
    }
    Ok(changes)
}

fn validate(key: &str) -> Result<(), NoGap> {
    let not_an_index = || NoGap(format!("Not an index: {key}"));
    let head = *key.as_bytes().first().ok_or_else(not_an_index)?;
    let len = integer_length(head).ok_or_else(not_an_index)?;
    if key.len() < len || !key.bytes().all(|c| DIGITS.contains(&c)) {
        return Err(not_an_index());
    }
    if key.ends_with('0') && key.len() > len {
        return Err(not_an_index());
    }
    Ok(())
}

/// How many characters the integer part has, from its first character.
fn integer_length(head: u8) -> Option<usize> {
    match head {
        b'a'..=b'z' => Some((head - b'a') as usize + 2),
        b'A'..=b'Z' => Some((b'Z' - head) as usize + 2),
        _ => None,
    }
}

/// The integer part and the fraction of a valid index.
fn split(key: &str) -> (&str, &str) {
    let len = integer_length(key.as_bytes()[0]).expect("validated");
    key.split_at(len)
}

fn digit(c: u8) -> usize {
    DIGITS.iter().position(|d| *d == c).expect("validated")
}

fn with_head(head: u8, digits: Vec<u8>) -> String {
    let mut bytes = Vec::with_capacity(digits.len() + 1);
    bytes.push(head);
    bytes.extend(digits);
    String::from_utf8(bytes).expect("ascii")
}

/// The next integer part, or None at the largest one.
fn increment_integer(x: &str) -> Option<String> {
    let head = x.as_bytes()[0];
    let mut digits = x.as_bytes()[1..].to_vec();
    for d in digits.iter_mut().rev() {
        let next = digit(*d) + 1;
        if next < RADIX {
            *d = DIGITS[next];
            return Some(with_head(head, digits));
        }
        *d = b'0';
    }
    // Every digit carried: the integer part gets longer
    match head {
        b'Z' => Some(FIRST.to_string()),
        b'z' => None,
        _ => {
            let next_head = head + 1;
            if next_head.is_ascii_lowercase() {
                digits.push(b'0');
            } else {
                digits.pop();
            }
            Some(with_head(next_head, digits))
        }
    }
}

/// The previous integer part, or None at the smallest one.
fn decrement_integer(x: &str) -> Option<String> {
    let head = x.as_bytes()[0];
    let mut digits = x.as_bytes()[1..].to_vec();
    for d in digits.iter_mut().rev() {
        let current = digit(*d);
        if current > 0 {
            *d = DIGITS[current - 1];
            return Some(with_head(head, digits));
        }
        *d = b'z';
    }
    // Every digit borrowed: the integer part gets shorter, or longer below `Z`
    match head {
        b'a' => Some("Zz".to_string()),
        b'A' => None,
        _ => {
            let next_head = head - 1;
            if next_head.is_ascii_uppercase() {
                digits.push(b'z');
            } else {
                digits.pop();
            }
            Some(with_head(next_head, digits))
        }
    }
}

/// A fraction between `a` and `b` (`b` absent: between `a` and the end), with no trailing
/// `0`. `a` is shorter than `b` or sorts before it.
fn midpoint(a: &str, b: Option<&str>) -> String {
    if let Some(b) = b {
        let mut n = 0;
        while let Some(&cb) = b.as_bytes().get(n) {
            if a.as_bytes().get(n).copied().unwrap_or(b'0') != cb {
                break;
            }
            n += 1;
        }
        if n > 0 {
            let rest_a = &a[n.min(a.len())..];
            return format!("{}{}", &b[..n], midpoint(rest_a, Some(&b[n..])));
        }
    }
    let digit_a = a.as_bytes().first().map(|c| digit(*c)).unwrap_or(0);
    let digit_b = b
        .and_then(|b| b.as_bytes().first())
        .map(|c| digit(*c))
        .unwrap_or(RADIX);
    if digit_b - digit_a > 1 {
        let mid = (digit_a + digit_b).div_ceil(2);
        return (DIGITS[mid] as char).to_string();
    }
    if let Some(b) = b {
        if b.len() > 1 {
            return b[..1].to_string();
        }
    }
    let rest_a = if a.is_empty() { "" } else { &a[1..] };
    format!("{}{}", DIGITS[digit_a] as char, midpoint(rest_a, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 最初のインデックスはa0である。
    #[test]
    fn initial_is_a0() {
        assert_eq!(initial(), "a0");
        assert_eq!(between(None, None).unwrap(), "a0");
    }

    // 末尾の後ろと先頭の前にインデックスが作られる。
    #[test]
    fn an_index_is_made_after_the_last_and_before_the_first() {
        assert_eq!(between(Some("a0"), None).unwrap(), "a1");
        assert_eq!(between(Some("az"), None).unwrap(), "b00");
        assert_eq!(between(Some("a0V"), None).unwrap(), "a1");
        assert_eq!(between(None, Some("a0")).unwrap(), "Zz");
        assert_eq!(between(None, Some("a1")).unwrap(), "a0");
        assert_eq!(between(None, Some("a0V")).unwrap(), "a0");
        assert_eq!(between(None, Some("Zz")).unwrap(), "Zy");
        assert_eq!(between(None, Some("Z0")).unwrap(), "Yzz");
        assert_eq!(between(None, Some("b00")).unwrap(), "az");
        assert_eq!(between(Some("Zz"), None).unwrap(), "a0");
        assert_eq!(between(Some("Yzz"), None).unwrap(), "Z0");
    }

    // 空間の両端でも、インデックスは作られるか、作れないと分かる。
    #[test]
    fn the_ends_of_the_space_make_an_index_or_say_they_cannot() {
        let largest = "zzzzzzzzzzzzzzzzzzzzzzzzzzz";
        assert_eq!(between(Some(largest), None).unwrap(), format!("{largest}V"));
        let smallest = "A00000000000000000000000000";
        assert_eq!(
            between(None, Some(&format!("{smallest}V"))).unwrap(),
            smallest
        );
        assert!(between(None, Some(smallest)).is_err());
    }

    // 2つのインデックスの間にインデックスが作られる。
    #[test]
    fn an_index_is_made_between_two_indices() {
        assert_eq!(between(Some("a0"), Some("a2")).unwrap(), "a1");
        assert_eq!(between(Some("a0"), Some("a1")).unwrap(), "a0V");
        assert_eq!(between(Some("a0V"), Some("a1")).unwrap(), "a0l");
        assert_eq!(between(Some("a0"), Some("a0V")).unwrap(), "a0G");
        assert_eq!(between(Some("a0"), Some("b00")).unwrap(), "a1");
        assert_eq!(between(Some("az"), Some("b00")).unwrap(), "azV");
        assert_eq!(between(Some("a0"), Some("a01V")).unwrap(), "a01");
    }

    // 末尾に1000個足しても、インデックスは3文字で済む。
    #[test]
    fn appending_a_thousand_indices_keeps_them_three_characters_long() {
        let v = generate_indices(1000);
        assert_eq!(v[61], "az");
        assert_eq!(v[62], "b00");
        assert_eq!(v[999].len(), 3);
        assert!(v.windows(2).all(|w| w[0] < w[1]));
    }

    // 同じ隙間に入れ続けても、インデックスはゆっくりしか伸びない。
    #[test]
    fn inserting_into_one_gap_grows_the_index_slowly() {
        let mut a = "a0".to_string();
        for _ in 0..60 {
            let next = between(Some(&a), Some("a1")).unwrap();
            assert!(a < next && next.as_str() < "a1");
            a = next;
        }
        // About five inserts per character
        assert!(a.len() <= 14, "{a}");
    }

    // 先頭に100回入れても、インデックスは作られる。
    #[test]
    fn inserting_at_the_top_a_hundred_times_keeps_making_indices() {
        let mut top = "a0".to_string();
        for _ in 0..100 {
            let next = between(None, Some(&top)).unwrap();
            assert!(next < top);
            top = next;
        }
    }

    // 旧い形のインデックスは受け付けない。
    #[test]
    fn an_index_of_the_old_scheme_is_refused() {
        assert!(between(Some("z0"), None).is_err());
        assert!(between(None, Some("y")).is_err());
        assert!(between(Some("a0"), Some("zzV")).is_err());
        assert!(between(Some("0"), None).is_err());
    }

    // 末尾が0の小数部は受け付けない。
    #[test]
    fn a_fraction_ending_in_zero_is_refused() {
        assert!(between(Some("a0V0"), None).is_err());
    }

    // 隙間に複数のインデックスが順に収まり、隙間を均等に使うので短い。
    #[test]
    fn many_indices_fit_in_a_gap_in_order_and_stay_short() {
        let v = between_many(Some("a0"), Some("a1"), 40).unwrap();
        assert_eq!(v.len(), 40);
        assert_ascending_between(Some("a0"), Some("a1"), &v);
        // Halving the gap each time, 40 indexes need two characters of fraction at most
        assert!(v.iter().all(|idx| idx.len() <= 4), "{v:?}");
    }

    // 末尾の後ろと先頭の前に複数のインデックスが作られる。
    #[test]
    fn many_indices_are_made_after_the_last_and_before_the_first() {
        let v = between_many(Some("a0"), None, 3).unwrap();
        assert_eq!(v, vec!["a1", "a2", "a3"]);
        let v = between_many(None, Some("a3"), 3).unwrap();
        assert_eq!(v, vec!["a0", "a1", "a2"]);
    }

    fn assert_ascending_between(before: Option<&str>, after: Option<&str>, indices: &[String]) {
        let mut previous = before.map(|s| s.to_string());
        for idx in indices {
            if let Some(p) = &previous {
                assert!(p.as_str() < idx.as_str(), "{p} should sort before {idx}");
            }
            previous = Some(idx.clone());
        }
        if let (Some(a), Some(p)) = (after, &previous) {
            assert!(p.as_str() < a, "{p} should sort before {a}");
        }
    }

    // 隣り合うインデックスの隙間でも、まとまりはその中に収まる。
    #[test]
    fn group_stays_inside_a_gap_of_adjacent_indices() {
        let indices = between_many(Some("a0U"), Some("a0V"), 25).unwrap();
        assert_eq!(indices.len(), 25);
        assert_ascending_between(Some("a0U"), Some("a0V"), &indices);
    }

    // 何度移動しても、隙間は使えるままである。
    #[test]
    fn gaps_stay_usable_after_many_moves() {
        let before = "a0".to_string();
        let mut after = "a1".to_string();
        for _ in 0..50 {
            let indices = between_many(Some(&before), Some(&after), 5).unwrap();
            assert_ascending_between(Some(&before), Some(&after), &indices);
            after = indices[0].clone();
        }
    }

    // 末尾に置いたまとまりは、最後の項目の後ろに並ぶ。
    #[test]
    fn group_at_the_end_sorts_after_the_last_item() {
        let indices = between_many(Some("a3"), None, 70).unwrap();
        assert_eq!(indices.len(), 70);
        assert_ascending_between(Some("a3"), None, &indices);
    }

    // 先頭に置いたまとまりは、最初の項目の前に並ぶ。
    #[test]
    fn group_at_the_top_sorts_before_the_first_item() {
        let indices = between_many(None, Some("a0"), 20).unwrap();
        assert_eq!(indices.len(), 20);
        assert_ascending_between(None, Some("a0"), &indices);
    }

    // 0個の項目にはインデックスが要らない。
    #[test]
    fn zero_items_need_no_index() {
        assert!(between_many(Some("a0"), Some("a1"), 0).unwrap().is_empty());
    }

    // 逆向きの隙間と幅のない隙間はエラーになる。
    #[test]
    fn a_reversed_or_empty_gap_is_an_error() {
        assert!(between_many(Some("a1"), Some("a0"), 1).is_err());
        assert!(between_many(Some("a1"), Some("a1"), 1).is_err());
        assert!(between(Some("a1"), Some("a1")).is_err());
    }

    // 連番のインデックスはa0から始まる。
    #[test]
    fn a_sequence_of_indices_starts_at_a0() {
        assert_eq!(generate_indices(3), vec!["a0", "a1", "a2"]);
        assert!(generate_indices(0).is_empty());
    }

    fn long(n: usize) -> String {
        format!("a0{}", "V".repeat(n))
    }

    // 上限より長いインデックスがなければ、何も変わらない。
    #[test]
    fn nothing_changes_while_no_index_is_longer_than_the_limit() {
        let at_limit = long(MAX_LEN - 2);
        assert_eq!(at_limit.len(), MAX_LEN);
        assert!(shorten(&["a0", &at_limit, "a1"]).unwrap().is_empty());
    }

    // 長いインデックスの一続きは、両隣の短いインデックスの間に付け直される。
    #[test]
    fn a_run_of_long_indices_is_reindexed_between_its_short_neighbours() {
        let run = [long(9), long(15), long(20)];
        let indices = ["a0", "a0V", &run[0], &run[1], &run[2], "a0W", "a1"];

        let changes = shorten(&indices).unwrap();

        let positions: Vec<usize> = changes.iter().map(|(p, _)| *p).collect();
        assert_eq!(positions, [2, 3, 4]);
        let fresh: Vec<String> = changes.into_iter().map(|(_, idx)| idx).collect();
        assert_ascending_between(Some("a0V"), Some("a0W"), &fresh);
        assert!(fresh.iter().all(|idx| idx.len() <= 5), "{fresh:?}");
    }

    // 片側に短いインデックスがなければ、ページの端までが付け直される。
    #[test]
    fn a_run_without_a_short_neighbour_is_reindexed_up_to_the_end_of_the_page() {
        let run = [long(20), long(9)];
        let changes = shorten(&[&run[0], &run[1], "a5"]).unwrap();
        let fresh: Vec<String> = changes.iter().map(|(_, idx)| idx.clone()).collect();
        assert_eq!(fresh, ["a3", "a4"]);

        let changes = shorten(&["a5", &run[0]]).unwrap();
        assert_eq!(changes, [(1, "a6".to_string())]);

        let changes = shorten(&[&run[0]]).unwrap();
        assert_eq!(changes, [(0, "a0".to_string())]);
    }

    // 離れた2つの一続きは、それぞれ付け直される。
    #[test]
    fn two_runs_apart_are_each_reindexed() {
        let run = [long(20), long(20)];
        let changes = shorten(&["a0", &run[0], "a1", &run[1], "a2"]).unwrap();
        assert_eq!(changes, [(1, "a0V".to_string()), (3, "a1V".to_string())]);
    }

    // 隣が旧い形のインデックスなら、付け直せない。
    #[test]
    fn a_run_next_to_an_index_of_the_old_scheme_cannot_be_reindexed() {
        let run = long(20);
        assert!(shorten(&["z0", &run, "a1"]).is_err());
    }
}
