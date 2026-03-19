# ASCII CLI Renderer Design

## Goal

Add a `dot-viewer ascii` CLI command that renders DOT files as Unicode box diagrams in the terminal — a first-class rendering target alongside the macOS and web viewers.

## Architecture

Three components working together:

```
.dot file
    │
    ├──► dot-parser (attribute extraction) ──► node metadata (shape, label, llm_model, etc.)
    │
    └──► dot-core (Graphviz "plain" format) ──► positioned coordinates for all nodes/edges
                                                    │
                                                    ▼
                                              Grid Mapper
                                           (float → char grid)
                                                    │
                                                    ▼
                                            ASCII Renderer
                                        (boxes, lines, arrows)
                                                    │
                                                    ▼
                                                 stdout
```

**Key insight:** Graphviz does the layout. We just map its output to characters. No custom layout algorithm needed.

## Parser Extension: Attribute Extraction

Add an `attributes` feature flag to `dot-parser`:

```toml
[features]
attributes = []
```

When enabled, the parser extracts key=value pairs from `[...]` blocks:

```rust
NodeDefinition { id, source_range, attributes: Vec<(String, String)> }
Edge { from, to, source_range, from_range, to_range, attributes: Vec<(String, String)> }
GraphAttribute { source_range, attributes: Vec<(String, String)> }
```

Attribute syntax to handle:
- `key=value` (bare identifier)
- `key="quoted value"` (with escape handling)
- `key=<HTML label>` (angle-bracket delimited, stored as raw string)

Consumer feature adoption:

| Crate | Features |
|-------|----------|
| `dot-core` (UniFFI) | `uniffi, attributes` |
| `dot-core-wasm` | `serde, attributes` |
| `dot-viewer-cli` | `attributes` |

## Graphviz `plain` Format

Expose `plain` output from dot-core alongside the existing SVG. Graphviz `plain` format provides:

```
graph <scale> <width> <height>
node <name> <x> <y> <width> <height> <label> <style> <shape> <color> <fillcolor>
edge <from> <to> <n> <x1> <y1> ... <xn> <yn> <label> <style> <color>
stop
```

This gives us everything needed: node positions, sizes, edge spline points, labels. The change to dot-core is minimal — parameterize the output format in the Graphviz FFI call (currently hardcoded to `"svg"`).

## Grid Mapper

Converts Graphviz floating-point coordinates to a character grid:

1. Parse the `plain` format into node/edge position structs
2. Scale coordinates to character units (each char cell ≈ 1 wide × 2 tall due to font aspect ratio)
3. Quantize node positions to grid cells
4. Route edges through grid cells avoiding node overlap

## ASCII Renderer

Draws the grid to a string:

**Node rendering by shape:**
- `ellipse` (default): `( NodeName )`
- `box`/`rect`: Box with `┌─┐│└─┘` borders
- `diamond`: `◇ NodeName`
- `Mdiamond`: `◆ NodeName`
- `Msquare`: `■ NodeName`
- Other shapes: Fall back to box with shape annotation

**Box content (default mode):**
```
┌────────────────┐
│   NodeName     │
│   shape / key  │
└────────────────┘
```

**Box content (verbose `-v`):**
```
┌────────────────┐
│   NodeName     │
│   box / sonnet │
│   prompt: ...  │
│   timeout: 30s │
└────────────────┘
```

**Edge rendering:**
- Vertical/horizontal lines: `│`, `─`
- Corners: `┌`, `┐`, `└`, `┘`
- Arrows: `▼`, `▲`, `►`, `◄`
- Edge labels rendered inline where space allows

**Color mode (`--color`):**
- Node borders: shape-dependent ANSI colors
- Labels: bold
- Edges: dim
- Edge labels: italic

## CLI Interface

New crate: `dot-viewer-cli`

```
dot-viewer ascii <file.dot>                # plain text output
dot-viewer ascii -v <file.dot>             # verbose (all attributes)
dot-viewer ascii --color <file.dot>        # ANSI colors
dot-viewer ascii --engine neato <file.dot>  # layout engine (default: dot)
```

Default output is plain text with no ANSI escapes — safe for piping and embedding.

Subcommand structure (`dot-viewer <subcommand>`) leaves room for future commands (e.g., `serve` for web, `validate`, etc.) without changing the interface.

## New Crate Structure

```
dot-viewer-cli/
├── Cargo.toml          # depends on dot-parser (attributes), dot-core
├── src/
│   ├── main.rs         # CLI arg parsing, file reading, orchestration
│   ├── plain.rs        # Graphviz plain format parser
│   ├── grid.rs         # Coordinate-to-character grid mapper
│   └── render.rs       # ASCII box/line renderer
```

## Testing

- **dot-parser**: Unit tests for attribute extraction (various quoting styles, edge cases)
- **plain.rs**: Unit tests parsing known `plain` format output into position structs
- **grid.rs**: Unit tests mapping float coordinates to character positions
- **render.rs**: Unit tests rendering individual nodes and edges
- **Integration**: Snapshot tests — known `.dot` file → expected ASCII output string

## Non-Goals

- Interactive TUI or scrollable output
- Pixel-perfect parity with Graphviz SVG rendering
- Color themes beyond basic ANSI
- Subcommands beyond `ascii` in initial version
