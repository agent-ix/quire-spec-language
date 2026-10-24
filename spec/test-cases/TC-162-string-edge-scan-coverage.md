---
id: TC-162
title: "The string-edge scan reports every unmarked string dispatch"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: verifies
---
# TC-162: The string-edge scan reports every unmarked string dispatch

## Description

Verify [FR-064](../functional/FR-064-restrict-string-dispatch-to-marked-edges.md)
through both QSL's adapter and the reusable scanner's library and command.
The scan reports unmarked string dispatch, respects valid consumer rules,
excludes test code, rejects branch-gating allow-list entries and preserves
source locations. Scope: FR-064-AC-1 through FR-064-AC-15.

The consumer procedures below are acceptance obligations for the extraction;
existing QSL-only tests do not establish consumer installation or adoption.
The structural detector covers literals and same-crate named string constants.
Variable-to-variable comparisons, external constants and indirect branch
dataflow retain the limitations stated by FR-064.

## Test Procedure

1. Build a fixture QSL-shaped source tree with one string comparison inside
   a `#[string_edge]`-marked function and one string `match` inside an
   unmarked function; run `xtask string-edge` against it.
2. Add a non-empty allow-list with the unmarked occurrence's file and item
   anchor and a display-only reason; re-run the scan. Remove the entry and
   re-run again. Use a separate comparison whose result only feeds a
   displayed message for this step, so it meets the allow-list rule.
3. Move the unmarked occurrence from step 1 into a `#[cfg(test)]` module,
   into a file under a `tests/` directory and into a file declared by
   `#[cfg(test)] mod x;` (separate fixtures); run the scan against each.
   Separately, a file named `tests.rs` or `*_tests.rs` that is not declared
   `#[cfg(test)]` is scanned.
4. Run the scan against an empty-report fixture and against the
   non-empty-report fixture; capture both exit codes.
5. Construct one allow-list entry whose comparison result feeds an `if`
   condition selecting between two code paths, and one entry whose result
   feeds only a logged message; submit each separately to the scanner.
6. For each of the five sites ADR-010 §4.3 names (`"allocation"`,
   `"quire.protocol.finite-global/v1"`, `"filament-canonical-json-1"`,
   `"quire.state.authority-adapter"`, and `"clock:"`), construct a branch-gating
   fixture and an allow-list entry naming it; submit each in turn. Run the
   real scan with marks ignored over the workspace and submit an entry for
   each named dispatch site still present in production. Preserve the
   existing real-site acceptance checks during extraction.
7. Confirm that the real `Makefile`'s `ci` target lists `string-edge` as a
   prerequisite and its recipe runs `cargo run --package xtask -- string-edge`.
   On a minimal fixture `Makefile` of the same aggregate/prerequisite shape,
   confirm a failed prerequisite fails the aggregate and a succeeding one
   does not.
8. Create an external workspace with `reader/src`, `emitter/src` and
   `unselected/src`, without QSL-named directories. Put one unmarked literal
   dispatch in each. Select only reader and emitter through the library
   request and binary flags. Repeat from a different working directory with
   absolute paths. Add a named string constant in reader and a same-named
   numeric constant and numeric comparison in emitter to check crate scope.
9. Exercise the fixtures from steps 1–5 through the public library and binary
   with consumer rules.
   Submit a branch-gating allow-list entry both before and after marking its
   function. Compare occurrence fields and errors with the QSL adapter.
10. Submit each invalid case independently: no roots; missing, non-directory,
    duplicate, overlapping and out-of-base roots; roots with no production
    Rust files; unreadable source; malformed Rust; malformed JSON; unknown,
    missing or wrongly typed rule fields; empty reasons; escaping rule
    paths; a source link outside the base; and a directory-link cycle.
    Capture the typed library error and actual command exit for every case.
    Use a permissions fixture or controlled reader failure that remains
    unreadable under the test user's privileges.
11. Execute the authoritative installation route and worked consumer runs
    below. Exercise a separate Rust client with the Git library dependency,
    calling the public API on the same clean and violating source. Inspect
    the client dependencies and files to establish that the scanner was
    referenced rather than copied.
12. Inspect the adapter call and the scanner crate dependencies to confirm
    there is one detector implementation and QSL-specific roots/rules stay
    in the adapter. Feed clean, violating and invalid-input fixtures through
    the adapter and direct library call. Confirm the existing lint
    prerequisite remains and extraction adds only the scanner crate's tests.

13. Mark one function `#[ix_trace_rs::string_edge]`, one
    `#[::ix_trace_rs::string_edge]` and one `#[string_edge]` imported from
    `ix-trace-rs`; scan with the library, the command and the
    adapter (FR-064-AC-12).
14. Compile a consumer crate with a marked function and run it; compile a
    second with `#[string_edge(anything)]` and read the compiler's errors
    (FR-064-AC-13).
15. Search the QSL workspace's manifests and production source for any attribute
    macro named `string_edge`, any re-export or alias of one and any crate whose
    only purpose is the marker, and run `cargo check` over every package so each
    marker's resolution is the compiler's (FR-064-AC-14).
16. Scan a fixture with a function marked `#[my_macros::string_edge]` and
    functions whose only mark is `// string_edge` or `/// string_edge`; read
    `cargo metadata` for the QSL workspace (FR-064-AC-15).

### Worked external consumer acceptance

The implementation's acceptance test creates a non-QSL crate outside the QSL
checkout, for example at `/tmp/string-edge-consumer`, with this manifest:

```toml
[package]
name = "consumer-example"
version = "0.1.0"
edition = "2021"
```

Its `edge-rules.json` is `{"allow_list": []}`. Its clean `src/lib.rs` is:

```rust
pub enum Kind { Accepted, Other }
pub fn accepted(kind: Kind) -> bool { matches!(kind, Kind::Accepted) }
```

The acceptance run installs and invokes the actual binary using existing
access to the private authoritative repository:

```sh
cargo install --git https://github.com/agent-ix/quire-spec-language --branch main qsl-string-edge
cd /tmp/string-edge-consumer
qsl-string-edge --base-dir . --crate-root src --rules edge-rules.json
```

It then replaces `src/lib.rs` with this one-line production violation and
repeats the same scan command:

```rust
pub fn accepted(kind: &str) -> bool { match kind { "accepted" => true, _ => false } }
```

The separate Rust library client's manifest references the authoritative
package using:

```toml
[dependencies]
qsl-string-edge = { git = "https://github.com/agent-ix/quire-spec-language", branch = "main" }
```

A consumer compiling marked intake functions uses the marker crate by
reference as well:

```toml
[dependencies]
ix-trace-rs = { git = "https://github.com/agent-ix/ix-trace-rs", branch = "main" }
```

These commands are the acceptance recipe, not a claim that the standalone
package has already been implemented or the installation run has passed.

## Expected Results

- Steps 1 and 3: only the unmarked production occurrence is reported, with
  its file and one-based line. Production files named `tests.rs` remain in
  scope; declared test modules and test directories do not.
- Step 2: adding the valid display-only entry suppresses its occurrence;
  removing the entry restores it without changing source.
- Step 4: the clean report exits zero and the violating report exits non-zero.
- Steps 5 and 6: branch-gating entries are rejected with source locations;
  display-only entries are accepted. Every named production dispatch shape
  is rejected as an allow-list entry, independent of the supplied reason.
- Step 7: the real gate invokes the adapter and propagates failure through
  the prerequisite. The fixture aggregate succeeds only when its prerequisite
  succeeds.
- Step 8 (FR-064-AC-7): only reader and emitter violations appear. Public
  library and command agree on file, line, item, branch classification and
  ordering; changing the working directory and equivalent path spelling
  does not change them. The numeric comparison in emitter is absent.
- Step 9 (FR-064-AC-8): all entry points agree on the fixture outcomes.
  A branch-gating entry produces a rule error even in a marked function;
  neither marking nor a consumer reason can suppress that error.
- Step 10 (FR-064-AC-9): every invalid input returns a typed library error
  identifying the affected input and binary exit `2`, without a clean result.
- Step 11 (FR-064-AC-10): installation succeeds by reference. The worked
  clean run exits `0`; the changed source exits `1`, reports `src/lib.rs:1`
  in item `accepted`, and classifies its string match as branch-gating.
  The separate library client returns the equivalent clean and violating
  reports without copied scanner sources or a QSL workspace layout.
- Step 12 (FR-064-AC-11): the adapter calls the public scanner with QSL's
  roots and rules and propagates its reports and failures. No duplicate
  detector or additional QSL gate is introduced.
- Step 13 (FR-064-AC-12): the three marked comparisons (`ix_trace_rs::`,
  `::ix_trace_rs::` and imported forms) are absent from every report and the
  unmarked one is present.
- Step 14 (FR-064-AC-13): the marked function behaves as the unmarked one; the
  argument case reports exactly one error with the message
  `#[string_edge] takes no arguments`.
- Step 15 (FR-064-AC-14): no such attribute macro, re-export, alias or crate is found,
  `cargo check` passes for every package, and each marker resolves to
  `ix_trace_rs::string_edge`.
- Step 16 (FR-064-AC-15): the `my_macros::string_edge`, comment and doc-comment
  fixtures are reported, and `ix-trace-rs` is a dependency of every package that
  uses the marker.
