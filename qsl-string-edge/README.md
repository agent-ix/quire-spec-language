# qsl-string-edge

The FR-064 string-edge scanner. It reports every string comparison or string
`match` outside a `#[string_edge]`-marked function in the crate source roots
you name. QSL's `xtask string-edge` and consumer repositories (IR, CG, driver)
run this one detector; do not copy its sources.

The scan is syntactic: it sees a string literal or a same-crate named
`const NAME: &str` on either side of a comparison, a `match` over such arms,
and the literal-taking string methods (`starts_with`, `ends_with`, `contains`,
`eq_ignore_ascii_case`, `strip_prefix`, `trim_start_matches`). It does not
resolve variable-to-variable comparisons, external constants or indirect
branch dataflow. Test code (`#[cfg(test)]` items, files declared by
`#[cfg(test)] mod x;`, anything under a `tests/` directory) is not scanned.
No other directory is skipped by name: a production `target/` or `.git`-named
module directory under a crate root is scanned.

## Install

From the authoritative private source, using your existing repository access:

```sh
cargo install --git https://github.com/agent-ix/quire-spec-language --branch main qsl-string-edge
```

As a library, in your `Cargo.toml`:

```toml
[dependencies]
qsl-string-edge = { git = "https://github.com/agent-ix/quire-spec-language", branch = "main" }
```

To mark an intake function you compile, depend on `ix-trace-rs`, which owns the marker (`use ix_trace_rs::string_edge;` or `#[ix_trace_rs::string_edge]`):

```toml
[dependencies]
ix-trace-rs = { git = "https://github.com/agent-ix/ix-trace-rs", branch = "main" }
```

Neither route needs a copy of the scanner, a fork, or QSL's `xtask`.

## Run

```sh
qsl-string-edge --base-dir <directory> --crate-root <source-directory> [--crate-root <source-directory> ...] --rules <rules.json>
```

- `--base-dir` is resolved against the current directory. Relative
  `--crate-root` and `--rules` paths are resolved against it, absolute ones are
  taken as is, and every root must lie inside the base.
- A crate root is the crate's source directory (`src`, `crates/consumer/src`),
  not its `Cargo.toml`. Each root is one crate's constant-resolution scope. Roots
  may not repeat or nest. A link whose target leaves the base is an error, a
  directory link back into its own ancestors is a cycle error, and a link into
  any selected root's tree is an error (it would report the same source twice).
  A link to a directory elsewhere in the base is followed and reported under the
  link's path.
- Files are reported relative to the base with `/` separators, one-based
  lines, with the enclosing item, sorted by file, line and item.
- Exit `0`: completed, nothing to report. Exit `1`: completed with violations,
  listed on stdout. Exit `2`: a request, rule, read or parse failure (message on
  stderr); a failed scan is never reported clean.

The rules file is a strict JSON object. `{"allow_list": []}` is an explicit
empty allow-list:

```json
{"allow_list": [{"file": "src/display.rs", "item": "display", "reason": "Display-only comparison of supplied text"}]}
```

An entry names a base-relative file, the enclosing item (`function` or
`Type::method`) and a non-empty reason, and admits a comparison that only
feeds a displayed or logged value. An entry at an item with any branch-gating
comparison (`if`/`while` condition, `match` scrutinee or arm, `&&`/`||` term,
`strip_prefix`) is a rule error even if the item is marked: make that function
a `#[string_edge]` edge that converts the string once instead.

## Worked consumer run

A crate outside any QSL checkout, `string-edge-consumer/`, with
`edge-rules.json` containing `{"allow_list": []}` and this `src/lib.rs`:

```rust
pub enum Kind { Accepted, Other }
pub fn accepted(kind: Kind) -> bool { matches!(kind, Kind::Accepted) }
```

```sh
cd string-edge-consumer
qsl-string-edge --base-dir . --crate-root src --rules edge-rules.json   # exit 0
```

Replacing `src/lib.rs` with an unmarked literal dispatch:

```rust
pub fn accepted(kind: &str) -> bool { match kind { "accepted" => true, _ => false } }
```

```sh
qsl-string-edge --base-dir . --crate-root src --rules edge-rules.json   # exit 1
# string-edge: 1 unmarked, unlisted occurrence(s):
#   src/lib.rs:1 in accepted (branch-gating)
```

## Library

```rust
use qsl_string_edge::{scan, EdgeRules, ScanRequest};

let report = scan(&ScanRequest {
    base_dir: ".".into(),
    crate_roots: vec!["src".into()],
    rules: EdgeRules::default(),
})?;
assert!(report.is_clean(), "{}", report.render());
```

`scan` returns the typed `Error` for any request, rule, read or parse failure.

## Tests

`cargo test -p qsl-string-edge` runs the detector, library, command and
fixture-consumer tests. The Git install and library-dependency route is a
network test, run on request against the branch that holds this crate:

```sh
QSL_STRING_EDGE_BRANCH=main cargo test -p qsl-string-edge --test it -- --ignored
```
