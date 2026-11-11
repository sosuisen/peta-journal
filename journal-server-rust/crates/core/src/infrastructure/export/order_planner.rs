//! Decides fractional indices for items in the order they appear in a file.
//! Items that already have an index keep it whenever possible (longest increasing subsequence of
//! existing indices); everything else gets a fresh index between its neighbours.

use crate::domain::fractional_index::{self, NoGap};

/// An index for every position, strictly increasing. Positions whose input was kept return that
/// same string, so callers can tell which ones were recomputed.
pub fn plan(existing_idx: &[Option<String>]) -> Result<Vec<String>, NoGap> {
    let n = existing_idx.len();
    let keep = longest_increasing_subsequence(existing_idx);
    let mut result: Vec<Option<String>> = (0..n)
        .map(|i| {
            if keep[i] {
                existing_idx[i].clone()
            } else {
                None
            }
        })
        .collect();
    let mut prev: Option<String> = None;
    for i in 0..n {
        if result[i].is_none() {
            let next = result[i + 1..].iter().find_map(|r| r.clone());
            result[i] = Some(fractional_index::between(prev.as_deref(), next.as_deref())?);
        }
        prev = result[i].clone();
    }
    Ok(result.into_iter().map(|r| r.unwrap_or_default()).collect())
}

/// Marks the positions forming the longest strictly increasing run of non-None indices.
fn longest_increasing_subsequence(idx: &[Option<String>]) -> Vec<bool> {
    let n = idx.len();
    let mut length = vec![0usize; n];
    let mut prev = vec![usize::MAX; n];
    let mut best_end: Option<usize> = None;
    for i in 0..n {
        let Some(cur) = &idx[i] else { continue };
        length[i] = 1;
        for j in 0..i {
            let Some(other) = &idx[j] else { continue };
            if other < cur && length[j] + 1 > length[i] {
                length[i] = length[j] + 1;
                prev[i] = j;
            }
        }
        if best_end.is_none_or(|b| length[i] > length[b]) {
            best_end = Some(i);
        }
    }
    let mut keep = vec![false; n];
    let mut i = best_end;
    while let Some(k) = i {
        keep[k] = true;
        i = if prev[k] == usize::MAX {
            None
        } else {
            Some(prev[k])
        };
    }
    keep
}

/// The indices with the ones grown too long replaced, as `fractional_index::shorten` does.
pub fn shortened(mut indices: Vec<String>) -> Result<Vec<String>, NoGap> {
    let as_str: Vec<&str> = indices.iter().map(String::as_str).collect();
    let changes = fractional_index::shorten(&as_str)?;
    for (position, idx) in changes {
        indices[position] = idx;
    }
    Ok(indices)
}

/// Fresh indices for every position, used when the index space is exhausted.
pub fn reindex_all(n: usize) -> Vec<String> {
    fractional_index::generate_indices(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[Option<&str>]) -> Vec<Option<String>> {
        v.iter().map(|o| o.map(|x| x.to_string())).collect()
    }

    // 既存のインデックスは保たれ、隙間が埋められる。
    #[test]
    fn keeps_existing_and_fills_gaps() {
        let r = plan(&s(&[Some("a0"), None, Some("a2")])).unwrap();
        assert_eq!(r[0], "a0");
        assert_eq!(r[2], "a2");
        assert!(r[1].as_str() > "a0" && r[1].as_str() < "a2");
    }

    // 順序と食い違うインデックスは付け直される。
    #[test]
    fn reorders_disagreeing_indices() {
        let r = plan(&s(&[Some("a2"), Some("a0"), Some("a1")])).unwrap();
        assert!(r[0] < r[1] && r[1] < r[2]);
        // The longest increasing run (a0, a1) is kept
        assert_eq!(r[1], "a0");
        assert_eq!(r[2], "a1");
    }

    // 長くなりすぎたインデックスは、取り込みで付け直される。
    #[test]
    fn indices_grown_too_long_are_replaced() {
        let long = format!("a0{}", "V".repeat(20));
        let r = shortened(vec!["a0".to_string(), long, "a1".to_string()]).unwrap();
        assert_eq!(r, vec!["a0", "a0V", "a1"]);
    }

    // インデックスのない項目はa0から番号が付く。
    #[test]
    fn items_without_an_index_are_numbered_from_a0() {
        let r = plan(&s(&[None, None])).unwrap();
        assert_eq!(r, vec!["a0", "a1"]);
    }
}
