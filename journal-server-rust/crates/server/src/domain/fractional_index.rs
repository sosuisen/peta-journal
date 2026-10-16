//! Fractional index strings for ordering items.
//! Lexicographic midpoints over the alphabet 0-9 A-Z a-z (62 characters, ASCII order).

const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const RADIX: usize = 62;
const MIN_CHAR: u8 = b'0';
const MID_CHAR: u8 = ALPHABET[RADIX / 2]; // 'V'
const FIRST: &str = "z0";

/// The gap between two indices cannot hold what was asked; the caller must reindex the page.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct NoGap(pub String);

/// The initial index for the first item.
pub fn initial() -> String {
    FIRST.to_string()
}

/// A new index that sorts between the two given indices.
/// If `before` is None the index sorts before `after`; if `after` is None it sorts after `before`.
pub fn between(before: Option<&str>, after: Option<&str>) -> String {
    match (before, after) {
        (None, None) => FIRST.to_string(),
        (None, Some(a)) => prepend(a),
        (Some(b), None) => append(b),
        (Some(b), Some(a)) => midpoint(b, a),
    }
}

/// `count` indices in ascending order that all sort between the two given indices.
pub fn between_many(
    before: Option<&str>,
    after: Option<&str>,
    count: usize,
) -> Result<Vec<String>, NoGap> {
    let mut out = Vec::with_capacity(count);
    if let (Some(b), Some(a)) = (before, after) {
        if b >= a {
            return Err(NoGap(format!("No gap between {b} and {a}")));
        }
        fill_between(b, a, count, &mut out)?;
        return Ok(out);
    }
    let mut current = after.or(before).map(|s| s.to_string());
    for _ in 0..count {
        current = Some(match after {
            None => between(current.as_deref(), None),
            Some(_) => between(None, current.as_deref()),
        });
        out.push(current.clone().unwrap_or_default());
    }
    if after.is_some() {
        out.reverse();
    }
    Ok(out)
}

/// Splits the gap in the middle first, so the indices grow in length as slowly as possible.
fn fill_between(before: &str, after: &str, count: usize, out: &mut Vec<String>) -> Result<(), NoGap> {
    if count == 0 {
        return Ok(());
    }
    let mid = midpoint(before, after);
    if mid.as_str() <= before || mid.as_str() >= after {
        return Err(NoGap(format!("No index between {before} and {after}")));
    }
    let left = (count - 1) / 2;
    fill_between(before, &mid, left, out)?;
    out.push(mid.clone());
    fill_between(&mid, after, count - 1 - left, out)
}

fn index_of(c: u8) -> usize {
    ALPHABET
        .iter()
        .position(|x| *x == c)
        .unwrap_or_else(|| panic!("Character not in alphabet: {}", c as char))
}

fn append(idx: &str) -> String {
    let bytes = idx.as_bytes();
    let last = bytes[bytes.len() - 1];
    let pos = index_of(last);
    if pos < RADIX - 1 {
        let mut s = idx[..idx.len() - 1].to_string();
        s.push(ALPHABET[pos + 1] as char);
        return s;
    }
    let mut s = idx.to_string();
    s.push(MID_CHAR as char);
    s
}

fn prepend(idx: &str) -> String {
    let bytes = idx.as_bytes();
    for i in (0..bytes.len()).rev() {
        let pos = index_of(bytes[i]);
        if pos > 0 {
            let mut s = idx[..i].to_string();
            s.push(ALPHABET[pos - 1] as char);
            return s;
        }
    }
    panic!("Cannot prepend before index: {idx}");
}

fn midpoint(a: &str, b: &str) -> String {
    let max_len = a.len().max(b.len());
    let pa = pad(a, max_len);
    let pb = pad(b, max_len);
    let mut result = String::new();
    let mut found = false;
    for i in 0..max_len {
        let ia = index_of(pa.as_bytes()[i]) as i64;
        let ib = index_of(pb.as_bytes()[i]) as i64;
        if !found {
            if ia == ib {
                result.push(ALPHABET[ia as usize] as char);
            } else if ib - ia > 1 {
                result.push(ALPHABET[(ia + (ib - ia) / 2) as usize] as char);
                found = true;
            } else {
                // Adjacent (or reversed) positions, carry to the next level
                result.push(ALPHABET[ia as usize] as char);
            }
        }
    }
    if !found {
        result.push(MID_CHAR as char);
    }
    result
}

/// Evenly spaced indices for re-indexing a page.
pub fn generate_indices(count: usize) -> Vec<String> {
    let mut indices = Vec::with_capacity(count);
    if count == 0 {
        return indices;
    }
    let mut current = FIRST.to_string();
    indices.push(current.clone());
    for _ in 1..count {
        current = append(&current);
        indices.push(current.clone());
    }
    indices
}

fn pad(s: &str, len: usize) -> String {
    let mut out = s.to_string();
    while out.len() < len {
        out.push(MIN_CHAR as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_is_z0() {
        assert_eq!(initial(), "z0");
        assert_eq!(between(None, None), "z0");
    }

    #[test]
    fn append_and_prepend() {
        assert_eq!(between(Some("z0"), None), "z1");
        assert_eq!(between(Some("zz"), None), "zzV");
        assert_eq!(between(None, Some("z0")), "y");
        assert_eq!(between(None, Some("z1")), "z0");
    }

    #[test]
    fn midpoints() {
        assert_eq!(between(Some("z0"), Some("z2")), "z1");
        let m = between(Some("z0"), Some("z1"));
        assert!(m.as_str() > "z0" && m.as_str() < "z1");
    }

    #[test]
    fn many_in_gap() {
        let v = between_many(Some("z0"), Some("z1"), 5).unwrap();
        assert_eq!(v.len(), 5);
        for w in v.windows(2) {
            assert!(w[0] < w[1]);
        }
        assert!(v[0].as_str() > "z0" && v[4].as_str() < "z1");
    }

    #[test]
    fn many_after_and_before() {
        let v = between_many(Some("z0"), None, 3).unwrap();
        assert_eq!(v, vec!["z1", "z2", "z3"]);
        let v = between_many(None, Some("z3"), 3).unwrap();
        assert_eq!(v, vec!["z0", "z1", "z2"]);
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

    #[test]
    fn group_stays_inside_a_narrow_gap() {
        // 17 items after "z0" used to reach "z0V", the index of the next item
        let indices = between_many(Some("z0"), Some("z0V"), 40).unwrap();
        assert_eq!(indices.len(), 40);
        assert_ascending_between(Some("z0"), Some("z0V"), &indices);
    }

    #[test]
    fn group_stays_inside_a_gap_of_adjacent_indices() {
        let indices = between_many(Some("z0U"), Some("z0V"), 25).unwrap();
        assert_ascending_between(Some("z0U"), Some("z0V"), &indices);
    }

    #[test]
    fn gaps_stay_usable_after_many_moves() {
        let before = "z0".to_string();
        let mut after = "z1".to_string();
        for _ in 0..50 {
            let indices = between_many(Some(&before), Some(&after), 5).unwrap();
            assert_ascending_between(Some(&before), Some(&after), &indices);
            after = indices[0].clone();
        }
    }

    #[test]
    fn group_at_the_end_sorts_after_the_last_item() {
        let indices = between_many(Some("z3"), None, 70).unwrap();
        assert_eq!(indices.len(), 70);
        assert_ascending_between(Some("z3"), None, &indices);
    }

    #[test]
    fn group_at_the_top_sorts_before_the_first_item() {
        let indices = between_many(None, Some("z0"), 20).unwrap();
        assert_eq!(indices.len(), 20);
        assert_ascending_between(None, Some("z0"), &indices);
    }

    #[test]
    fn zero_items_need_no_index() {
        assert!(between_many(Some("z0"), Some("z1"), 0).unwrap().is_empty());
    }

    #[test]
    fn no_gap() {
        assert!(between_many(Some("z1"), Some("z0"), 1).is_err());
        assert!(between_many(Some("z1"), Some("z1"), 1).is_err());
    }

    #[test]
    fn generates_sequence() {
        assert_eq!(generate_indices(3), vec!["z0", "z1", "z2"]);
        assert!(generate_indices(0).is_empty());
    }
}
