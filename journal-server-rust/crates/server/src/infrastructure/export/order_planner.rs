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
        .map(|i| if keep[i] { existing_idx[i].clone() } else { None })
        .collect();
    let mut prev: Option<String> = None;
    for i in 0..n {
        if result[i].is_none() {
            let next = result[i + 1..].iter().find_map(|r| r.clone());
            let before = prev.as_deref();
            let after = next.as_deref();
            if let (Some(b), Some(a)) = (before, after) {
                if b >= a {
                    return Err(NoGap(format!("No gap between {b} and {a}")));
                }
            }
            let idx = fractional_index::between(before, after);
            if let Some(b) = before {
                if idx.as_str() <= b {
                    return Err(NoGap(format!("No index after {b}")));
                }
            }
            if let Some(a) = after {
                if idx.as_str() >= a {
                    return Err(NoGap(format!("No index before {a}")));
                }
            }
            result[i] = Some(idx);
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
        i = if prev[k] == usize::MAX { None } else { Some(prev[k]) };
    }
    keep
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

    #[test]
    fn keeps_existing_and_fills_gaps() {
        let r = plan(&s(&[Some("z0"), None, Some("z2")])).unwrap();
        assert_eq!(r[0], "z0");
        assert_eq!(r[2], "z2");
        assert!(r[1].as_str() > "z0" && r[1].as_str() < "z2");
    }

    #[test]
    fn reorders_disagreeing_indices() {
        let r = plan(&s(&[Some("z2"), Some("z0"), Some("z1")])).unwrap();
        assert!(r[0] < r[1] && r[1] < r[2]);
        // The longest increasing run (z0, z1) is kept
        assert_eq!(r[1], "z0");
        assert_eq!(r[2], "z1");
    }

    #[test]
    fn all_new() {
        let r = plan(&s(&[None, None])).unwrap();
        assert_eq!(r, vec!["z0", "z1"]);
    }
}
