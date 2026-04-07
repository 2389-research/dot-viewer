# dippin-parser

A Rust parser and DOT exporter for the [Dippin DSL](https://github.com/2389-research/dippin-lang),
a higher-level authoring format for AI agent workflows.

## Status

Pre-1.0. Public types are `#[non_exhaustive]` to allow additive evolution.

## Usage

```rust
use dippin_parser::{parse, parse_to_dot_with_options, ExportOptions, RankDir};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string("workflow.dip")?;
    let _wf = parse(&src, "workflow.dip")?;

    let mut opts = ExportOptions::default();
    opts.include_prompts = true;
    opts.rank_dir = RankDir::LeftRight;
    let dot = parse_to_dot_with_options(&src, "workflow.dip", &opts)?;
    println!("{dot}");
    Ok(())
}
```

## Features

- `serde` — derives `Serialize`/`Deserialize` on every public IR type.

## Relationship to dippin-lang

This crate tracks the upstream Go implementation at
[`2389-research/dippin-lang`](https://github.com/2389-research/dippin-lang).
Behavioral parity is maintained through ported test fixtures in `testdata/`.

## License

MIT OR Apache-2.0
