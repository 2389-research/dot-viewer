// ABOUTME: Library usage example: parse a .dip file and emit DOT.
// ABOUTME: Run with `cargo run -p dippin-parser --example parse_and_export -- <path>`.

use dippin_parser::{parse_to_dot_with_options, ExportOptions, RankDir};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: parse_and_export <path.dip>");
            return ExitCode::from(64);
        }
    };

    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {}: {}", path, e);
            return ExitCode::from(66);
        }
    };

    let mut opts = ExportOptions::default();
    opts.include_prompts = true;
    opts.rank_dir = RankDir::TopBottom;

    match parse_to_dot_with_options(&src, &path, &opts) {
        Ok(dot) => {
            println!("{dot}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            for d in e.diagnostics() {
                eprintln!("{}", d.render());
            }
            ExitCode::from(65)
        }
    }
}
