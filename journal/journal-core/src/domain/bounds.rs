//! The place of a card or a callout on an atlas: `x,y,width,height`.
//!
//! The type is the one of `peta-atlas`, which the clients read the value with too. The text is
//! parsed once at the boundary, so the code inside never sees a wrong one.

pub use peta_atlas::{Bounds, NotBounds};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::number_text::format_number;

    // 4つの数は、空白を除き、数のプロパティと同じ書き方で書き直される。
    #[test]
    fn four_numbers_are_written_back_without_spaces_as_number_properties_are() {
        let bounds: Bounds = " 10, 20.5,160 ,-56 ".parse().unwrap();
        assert_eq!(bounds.to_string(), "10,20.5,160,-56");
    }

    // 大きな数、小さな数、負の数も、数のプロパティと同じ書き方になる。
    #[test]
    fn every_number_is_written_as_a_number_property_shows_it() {
        for n in [0.0, -0.5, 3.0, 1e15, 1e15 - 1.0, -1e15, 1e21, 0.1 + 0.2] {
            let bounds: Bounds = format!("{n},0,0,0").parse().unwrap();
            assert_eq!(bounds.to_string(), format!("{},0,0,0", format_number(n)));
        }
    }

    // 4つの有限な数でなければ、boundsにならない。
    #[test]
    fn anything_but_four_finite_numbers_is_not_bounds() {
        for text in ["1,2,3", "1,2,3,4,5", "1,2,x,4", "", "1,2,3,inf"] {
            assert!(text.parse::<Bounds>().is_err(), "{text}");
        }
    }
}
