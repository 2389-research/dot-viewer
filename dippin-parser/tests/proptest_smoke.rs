// ABOUTME: Property tests for the dippin parser — panic-freedom on arbitrary input
// ABOUTME: plus structural round-trip invariants over the canonical valid fixtures.

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

#[test]
fn round_trip_structural_invariants() {
    let fixtures = ["valid_minimal.dip", "multi_provider.dip", "ask_and_execute.dip"];
    for f in fixtures {
        let path = format!("{}/testdata/{}", env!("CARGO_MANIFEST_DIR"), f);
        let src = std::fs::read_to_string(&path).expect("fixture file readable");
        let wf = dippin_parser::parse(&src, f).expect("fixture parses");
        let dot = wf.to_dot(&dippin_parser::ExportOptions::default());
        // Cannot re-parse DOT as .dip, so just assert structural invariants.
        assert!(!wf.nodes.is_empty(), "{f}: nodes should be non-empty");
        assert!(dot.contains("digraph"), "{f}: DOT export should contain digraph");
    }
}
