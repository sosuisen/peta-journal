//! Numbers as the properties and the files show them.

/// A number as a number property shows it: an integer without a fraction, anything else as
/// Java's `String.valueOf(double)` writes it, which the files from the Java server hold.
pub fn format_number(n: f64) -> String {
    if n == n.floor() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format_double(n)
    }
}

/// Formats a non-integral double the way Java's `String.valueOf(double)` does for common values.
fn format_double(n: f64) -> String {
    let s = format!("{n}");
    // Rust's `{}` never writes an exponent, so only the special values have no '.'
    if s.contains('.') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 整数は小数部なしで、それ以外はJavaのString.valueOf(double)の形で書かれる。
    #[test]
    fn an_integer_is_written_without_a_fraction_and_the_rest_as_java_writes_them() {
        assert_eq!(format_number(3.0), "3");
        assert_eq!(format_number(-56.0), "-56");
        assert_eq!(format_number(20.5), "20.5");
        assert_eq!(format_number(f64::INFINITY), "inf");
        assert_eq!(format_number(f64::NAN), "NaN");
        // Past 1e15 an integer no longer fits the way Java writes it, so it keeps a fraction
        assert_eq!(format_number(1e15), "1000000000000000.0");
        assert_eq!(format_number(1e15 - 1.0), "999999999999999");
    }
}
