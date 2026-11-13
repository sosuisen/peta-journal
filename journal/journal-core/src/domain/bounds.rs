//! The place of a card or a callout on an atlas: `x,y,width,height`.

use std::fmt;
use std::str::FromStr;

use super::number_text::format_number;

/// Four finite numbers `x,y,width,height`. The text is parsed once at the boundary, so the
/// code inside never sees a wrong one. It is written back without spaces, each number as a
/// number property shows it, so the front matter and the metadata line of an item hold it as
/// one plain value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds([f64; 4]);

/// The text is not four finite numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotBounds(String);

impl fmt::Display for NotBounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Bounds must be four numbers x,y,width,height: {}",
            self.0
        )
    }
}

impl std::error::Error for NotBounds {}

impl FromStr for Bounds {
    type Err = NotBounds;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let numbers = text
            .split(',')
            .map(|n| n.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
            .collect::<Option<Vec<f64>>>();
        match numbers.as_deref() {
            Some(&[x, y, width, height]) => Ok(Bounds([x, y, width, height])),
            _ => Err(NotBounds(text.to_string())),
        }
    }
}

impl fmt::Display for Bounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self.0.map(format_number).join(",");
        f.write_str(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 4つの数は、空白を除き、数のプロパティと同じ書き方で書き直される。
    #[test]
    fn four_numbers_are_written_back_without_spaces_as_number_properties_are() {
        let bounds: Bounds = " 10, 20.5,160 ,-56 ".parse().unwrap();
        assert_eq!(bounds.to_string(), "10,20.5,160,-56");
    }

    // 4つの有限な数でなければ、boundsにならない。
    #[test]
    fn anything_but_four_finite_numbers_is_not_bounds() {
        for text in ["1,2,3", "1,2,3,4,5", "1,2,x,4", "", "1,2,3,inf"] {
            assert!(text.parse::<Bounds>().is_err(), "{text}");
        }
    }
}
