// ABOUTME: Duration newtype with Go-style parsing (e.g., "30s", "5m", "1h30m").
// ABOUTME: Replaces stringly-typed duration fields in IR.

use std::fmt;
use std::time::Duration as StdDuration;

/// A duration value with Go-style parsing semantics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Duration(pub StdDuration);

impl Duration {
    /// Returns true if this duration represents zero time.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Parse a Go-style duration literal like "1h30m", "500ms", "5s".
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Ok(Duration(StdDuration::ZERO));
        }
        let mut total = StdDuration::ZERO;
        let mut rest = s;
        while !rest.is_empty() {
            let num_end = rest
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .ok_or_else(|| format!("invalid duration: {}", s))?;
            if num_end == 0 {
                return Err(format!("invalid duration: {}", s));
            }
            let n: f64 = rest[..num_end]
                .parse()
                .map_err(|_| format!("invalid duration number: {}", &rest[..num_end]))?;
            let unit_end = rest[num_end..]
                .find(|c: char| c.is_ascii_digit())
                .map(|p| num_end + p)
                .unwrap_or(rest.len());
            let unit = &rest[num_end..unit_end];
            let nanos = match unit {
                "ns" => n,
                "us" | "µs" => n * 1_000.0,
                "ms" => n * 1_000_000.0,
                "s" => n * 1_000_000_000.0,
                "m" => n * 60.0 * 1_000_000_000.0,
                "h" => n * 3600.0 * 1_000_000_000.0,
                "" => return Err(format!("missing duration unit in: {}", s)),
                _ => return Err(format!("unknown duration unit: {}", unit)),
            };
            total += StdDuration::from_nanos(nanos as u64);
            rest = &rest[unit_end..];
        }
        Ok(Duration(total))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_zero() {
            return write!(f, "0s");
        }

        let total_secs = self.0.as_secs();
        let sub_ms = self.0.subsec_millis() as u64;

        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        let mut wrote = false;
        if hours > 0 {
            write!(f, "{}h", hours)?;
            wrote = true;
        }
        if minutes > 0 {
            write!(f, "{}m", minutes)?;
            wrote = true;
        }
        if seconds > 0 {
            write!(f, "{}s", seconds)?;
            wrote = true;
        }
        if sub_ms > 0 {
            write!(f, "{}ms", sub_ms)?;
            wrote = true;
        }
        if !wrote {
            // Sub-millisecond remainder only (e.g. microseconds); fall back to 0s.
            write!(f, "0s")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_seconds() {
        assert_eq!(Duration::parse("30s").unwrap().0, StdDuration::from_secs(30));
    }

    #[test]
    fn parses_minutes() {
        assert_eq!(Duration::parse("5m").unwrap().0, StdDuration::from_secs(300));
    }

    #[test]
    fn parses_hours_and_minutes() {
        assert_eq!(
            Duration::parse("1h30m").unwrap().0,
            StdDuration::from_secs(3600 + 1800)
        );
    }

    #[test]
    fn parses_milliseconds() {
        assert_eq!(
            Duration::parse("500ms").unwrap().0,
            StdDuration::from_millis(500)
        );
    }

    #[test]
    fn empty_is_zero() {
        assert!(Duration::parse("").unwrap().is_zero());
    }

    #[test]
    fn rejects_unknown_unit() {
        assert!(Duration::parse("5x").is_err());
    }

    #[test]
    fn rejects_missing_unit() {
        assert!(Duration::parse("5").is_err());
    }

    #[test]
    fn display_round_trip_seconds() {
        assert_eq!(Duration::parse("30s").unwrap().to_string(), "30s");
    }

    #[test]
    fn display_round_trip_minutes() {
        assert_eq!(Duration::parse("5m").unwrap().to_string(), "5m");
    }

    #[test]
    fn display_round_trip_hours_and_minutes() {
        assert_eq!(Duration::parse("1h30m").unwrap().to_string(), "1h30m");
    }

    #[test]
    fn display_round_trip_hours_only() {
        assert_eq!(Duration::parse("2h").unwrap().to_string(), "2h");
    }

    #[test]
    fn display_round_trip_plain_seconds() {
        assert_eq!(Duration::parse("30s").unwrap().to_string(), "30s");
    }

    #[test]
    fn display_round_trip_milliseconds() {
        assert_eq!(Duration::parse("500ms").unwrap().to_string(), "500ms");
    }

    #[test]
    fn display_zero_is_zero_seconds() {
        assert_eq!(Duration::parse("").unwrap().to_string(), "0s");
    }

    #[test]
    fn display_round_trip_full_compound() {
        assert_eq!(Duration::parse("1h2m3s").unwrap().to_string(), "1h2m3s");
    }
}
