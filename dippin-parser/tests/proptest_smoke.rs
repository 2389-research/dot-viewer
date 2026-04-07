// ABOUTME: Property test ensuring the parser never panics on arbitrary input.
// ABOUTME: Generates random ASCII and unicode strings and asserts parse() returns Ok or Err — never panic.

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn parse_does_not_panic_on_random_ascii(s in "[\\x20-\\x7e\\n\\t]{0,2048}") {
        let _ = dippin_parser::parse(&s, "fuzz.dip");
    }

    #[test]
    fn parse_does_not_panic_on_random_unicode(s in "\\PC{0,512}") {
        let _ = dippin_parser::parse(&s, "fuzz.dip");
    }
}
