# dippin-parser test fixtures

These `.dip` files exercise the parser. Each is described below.

| File | Purpose |
|---|---|
| `valid_minimal.dip` | Smallest valid workflow with one human and one agent node. |
| `valid_minimal_v2.dip` | Variant of `valid_minimal.dip` testing alternate syntax. |
| `multi_provider.dip` | Multiple agents using different LLM providers. |
| `ask_and_execute.dip` | Complex workflow with parallel, fan_in, conditionals, restarts. |
| `ask_and_execute.dot` | Golden DOT output for `ask_and_execute.dip`. |
| `unicode.dip` | UTF-8 regression coverage (multi-byte chars in identifiers, prompts, labels). |

Files starting with `valid_` must parse without errors. The corresponding
integration tests live in `dippin-parser/tests/integration_tests.rs`.
