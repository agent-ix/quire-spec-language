---
id: FR-064
title: "Restrict string dispatch to marked edges"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-006
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
---
# FR-064: Restrict string dispatch to marked edges

## Description

ADR-012 §9 fixes one rule: a string may select semantics only at a listed
edge (source lexing, a typed wire reader, a protocol-artifact or state-input
intake reader, or CLI argument parsing), and at that edge one total
conversion turns the string into a closed enum or a typed identity, or
refuses it with a typed cause. After the edge, no code compares a string to
choose behaviour. The attribute `#[string_edge]` marks an edge function. It is owned by the
neutral crate `ix-trace-rs` (agent-ix/ix-trace-rs), not by QSL. QSL SHALL run
a scan that reports every string comparison or string `match` outside a
marked function in the selected crates' non-test code, so this rule is enforced
by a running check rather than by convention alone.

The authoritative scanner SHALL be a standalone Rust library and command
that consumers can obtain from this repository and run against their own
crate roots. QSL's `xtask string-edge` SHALL delegate to that library with
QSL's roots and edge rules.

## Inputs

- Explicit consumer crate source roots and a base directory for resolving
  source paths, rule entries and report locations. QSL supplies its own roots
  through its adapter.
- The selected crates' source (non-test code; `#[cfg(test)]` items, files declared
  by `#[cfg(test)] mod x;` and files under a `tests/` directory are out of
  scope for the scan; a file is never out of scope by its name alone).
- The `#[string_edge]` attribute of `ix-trace-rs` applied to an edge function.
- A checked-in allow-list of string comparisons that compare a user-supplied
  value and select no behaviour on its content.

## Outputs

- A library report and a command report listing every string comparison or string
  `match` found outside a `#[string_edge]`-marked function and not covered by
  the allow-list, with the file and line of each.
- A command exit of zero only after a completed scan with no violations;
  non-zero for violations or a scan that could not complete.

## Behavior

Detector scope: a comparison is branch-gating when it feeds an
`if`/`while` condition or `match` scrutinee/guard, is a term of a `&&`/`||`
chain, is a match arm's own value, or is a `strip_prefix` call.

### The marker attribute

The marker is the attribute `#[string_edge]`, written as `string_edge`, as
`ix_trace_rs::string_edge` or as `::ix_trace_rs::string_edge`. The marker has no comment or doc-comment form.

Contract QSL relies on (owned by `ix-trace-rs`'s own specification; Linear
QSL-953 tracks its publication): the attribute is a proc-macro attribute of
`ix-trace-rs` that applies to a function, takes no argument, expands to the
annotated item unchanged, adds no runtime behaviour, and reports an argument as
one compile error with the message `#[string_edge] takes no arguments` while
retaining the item. `ix-trace-rs` has no normal dependency and none on QSL, a
model, IR or a code-generation crate, so a consumer depends on it directly.

- The scan SHALL treat a function as marked when it carries an attribute whose
  path is `string_edge`, `ix_trace_rs::string_edge` or
  `::ix_trace_rs::string_edge`.
- The scan SHALL NOT treat any other path, a crate-qualified path of another
  crate included, as a marker.
- The scan SHALL treat a marker as covering only the function it is applied to,
  never a file or a crate.
- The QSL workspace SHALL import the attribute from `ix-trace-rs` and declare
  that dependency in the manifest of each package that uses it.
- The QSL workspace SHALL define no attribute macro named `string_edge` and no
  re-export or alias of one.

A consumer imports the attribute from `ix-trace-rs` (`use
ix_trace_rs::string_edge;`) or writes `#[ix_trace_rs::string_edge]`; a consumer
that keeps its own copy of the marker is outside what the scan can detect by path.

### What the scan reports

`xtask string-edge` SHALL scan every QSL crate's non-test source for:

- an equality or ordering comparison between a `&str`/`String` value and a
  string literal or another `&str`/`String` value, and
- a `match` expression whose scrutinee is a `&str`/`String` value,

and SHALL report each occurrence found outside a function carrying
`#[string_edge]`, unless that occurrence is named in the allow-list.

### The allow-list

The allow-list SHALL be a checked-in file naming each allowed occurrence by
file and line (or by a stable anchor if the file is expected to move), with a
one-line reason. An allow-list entry SHALL name only a comparison of a
user-supplied value that selects no program behaviour based on its content
-- for example, a `debug_assert_eq!` in production code that compares a
rendered user-supplied value against its expected display text purely as an
internal consistency check, compiled out of release builds and reachable by
no code path in a release binary. An entry that gates a branch, a lookup or
a dispatch decision on the compared string's value SHALL NOT be added to the
allow-list; such an occurrence is refused by making its function an edge,
marking it `#[string_edge]`, and converting the string once, or by moving
the dispatch to a closed enum or typed identity.

`xtask string-edge` SHALL reject an allow-list entry whose comparison result
feeds an `if` condition or a `match` scrutinee that selects between
different code paths, reporting the entry's file and line and refusing to
treat it as allowed; it SHALL accept an entry only when the comparison's
result feeds a non-branching sink (a logged or displayed message, a
diagnostic payload, a test assertion, or code compiled out of the checked
build). None of the five production dispatch sites ADR-010 §4.3 names (the
`"allocation"` relationship-category string, the
`"quire.protocol.finite-global/v1"` profile string, the
`"filament-canonical-json-1"` canonicalization string, the
`"quire.state.authority-adapter"` adapter string, and the `clock:` prefix)
can be accepted into the allow-list, because each one gates a branch; where
a site is not yet converted by its owning family's ticket, `xtask
string-edge` SHALL continue to report it as an unresolved violation rather
than accept it into the allow-list.

### Gate placement

`xtask string-edge` SHALL run in the lint gate.

### Reusable consumer interface

The standalone package and binary SHALL be named `qsl-string-edge`. Its
library SHALL accept a request containing a base directory, one or more
crate source directories, and edge rules. Each source directory defines one
crate's constant-resolution scope; supplying multiple roots SHALL NOT make
a constant declared in one crate visible to the detector in another crate.
The scanner SHALL use only the caller's supplied roots and rules, without
requiring QSL crate names, a QSL checkout as the current directory, or a
QSL compiler/runtime dependency. Each root denotes the source directory
itself, such as `src` or `crates/consumer/src`, not its `Cargo.toml`.

The command SHALL expose the same request as:

```text
qsl-string-edge --base-dir <directory> --crate-root <source-directory> [--crate-root <source-directory> ...] --rules <rules.json>
```

`--base-dir` SHALL resolve relative to the invocation's current directory;
relative crate-root and rules-file paths SHALL resolve relative to that base.
Absolute inputs SHALL be accepted. A source root SHALL be contained within
the base directory after path resolution. The scanner SHALL reject a source
file or directory link that escapes the base, and SHALL NOT follow a
directory-link cycle. Reports SHALL identify files relative to the base
with `/` separators, using one-based source lines and the enclosing item
anchor. Equivalent absolute and relative inputs SHALL produce the same
ordered occurrences, sorted by file, line and item. Duplicate or overlapping
crate roots SHALL be rejected so the scan cannot confuse crate scopes or
report an occurrence twice.

The library SHALL return a structured report containing the occurrences
(file, line, item and branch-gating classification). It SHALL return a typed
error identifying the affected input when a request, rule, source read or
Rust parse fails. The command SHALL exit `0` for a completed clean report,
`1` for a completed report containing violations, and `2` for a request,
rule, read or parse failure. An empty root list, missing or non-directory
root, or selection containing no production Rust files SHALL be a request
failure. A failed or partial scan SHALL NOT emit a clean result. Both
entry points SHALL apply the same detector, marker, test exclusion and
allow-list logic.

### Consumer edge rules

The command SHALL read edge rules from a strict JSON object with an
`allow_list` array; an empty array is an explicit empty allow-list:

```json
{"allow_list": [{"file": "src/display.rs", "item": "display", "reason": "Display-only comparison of supplied text"}]}
```

Each entry SHALL contain a base-relative file path, an enclosing item anchor
and a non-empty reason. These are the stable-anchor alternative already
allowed above. Unknown fields, wrong field types, missing fields, malformed
JSON and paths outside the base SHALL be rule errors. The library SHALL
accept the equivalent typed rule value without requiring a rules file.
The marker is `#[string_edge]`, including its qualified form
`#[ix_trace_rs::string_edge]`; consumers that compile marked functions SHALL
obtain the `ix-trace-rs` crate by Cargo dependency on its repository
(agent-ix/ix-trace-rs). A marker identifies the one total conversion at a
permitted edge; it is not a consumer-defined exemption for an entire crate or
file.

Consumer rules SHALL obey the allow-list restrictions above. An entry
covering any detected branch-gating occurrence in its item SHALL be refused
as a rule error, even if that item is marked. The scanner SHALL check that
condition before applying marker or allow-list suppression. QSL-specific
roots and rules SHALL reside in QSL's adapter; the reusable scanner SHALL
NOT import them as defaults. Consumers own their edge inventories and
choice of CI invocation.

### Installation and acceptance boundary

The consumer documentation SHALL give this installation route from the
authoritative private source, using existing repository access:

```text
cargo install --git https://github.com/agent-ix/quire-spec-language --branch main qsl-string-edge
```

A library consumer SHALL be able to use the same package through a Cargo
git dependency with `branch = "main"`. Neither route SHALL require copying
scanner sources, maintaining a scanner fork, or building QSL's `xtask` as
the consumer command. The documented consumer example SHALL run the
installed binary from outside QSL against a non-QSL Rust crate, with its
own rule file, and show both a clean run and an unmarked dispatch failure.
The implementation acceptance run SHALL execute that install and both
invocations; prose examples alone do not establish the route works.

QSL's adapter SHALL retain the existing `string-edge` lint prerequisite
and propagate scanner failures. Extraction SHALL add no QSL gate beyond
the scanner crate's own tests. Consumer adoption in IR, CG and driver is
separate work. Reuse preserves the existing structural detector's coverage
and documented limitations; it does not claim to resolve variable-to-variable
comparisons, external constants or indirect branch dataflow.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-064-AC-1 | Given a QSL source tree with one string comparison inside a function marked `#[string_edge]` and one string `match` inside an unmarked function, `xtask string-edge` reports exactly the unmarked occurrence, naming its file and line, and does not report the marked occurrence. | Test |
| FR-064-AC-2 | Given an unmarked occurrence whose location matches an allow-list entry, `xtask string-edge` does not report it; removing that allow-list entry causes the same occurrence to be reported on the next scan, with no change to the source. | Test |
| FR-064-AC-3 | `xtask string-edge` scans no file under a `tests/` directory and no `#[cfg(test)]` module; a string `match` placed only inside such a module is not reported even when unmarked and not allow-listed. | Test |
| FR-064-AC-4 | `xtask string-edge` exits non-zero when its report is non-empty and exits zero when its report is empty; a test constructs one fixture tree of each shape and asserts both exit codes. | Test |
| FR-064-AC-5 | Given one allow-list entry whose comparison result feeds an `if`/`match` condition that selects between two different code paths, and one entry whose comparison result feeds only a logged message, `xtask string-edge` rejects the first (naming its file and line) and accepts the second. A test attempts to add each of the five ADR-010 §4.3 production dispatch sites (named in this requirement's Behavior) as an allow-list entry and asserts the tool rejects all five; a wrong implementation that allow-lists all five to force a clean scan does not satisfy this criterion. | Test |
| FR-064-AC-6 | The lint gate's own target list (the `Makefile`) names `string-edge` as a prerequisite of the gate it runs in, whose own recipe runs `cargo run --package xtask -- string-edge`, and `xtask string-edge`'s non-zero exit propagates to that gate's own exit through `make`'s ordinary prerequisite-failure semantics -- a `.PHONY` aggregate target depending on a target whose recipe can fail. Both halves are checked directly: a grep-shaped check over the real `Makefile` confirms the prerequisite naming and the recipe, and a minimal fixture `Makefile` of the same aggregate/prerequisite shape demonstrates a failed prerequisite failing the aggregate target (and a succeeding one not failing it). | Test |
| FR-064-AC-7 | Given two non-QSL crate source roots and a third unselected crate containing a violation, the library and binary report only occurrences in the selected roots with the same file, one-based line, item, branch classification and ordering. Repeating with equivalent absolute paths from another working directory produces identical occurrences. A named string constant in one selected crate does not classify a same-named non-string constant in the other as string dispatch. | Test |
| FR-064-AC-8 | Given consumer rules and fixtures for marked intake, test exclusion, an allowed display-only comparison and a branch-gating comparison, the library and binary apply the same marker/exclusion/allow-list results as the QSL adapter. Removing the display entry restores its occurrence. Adding an entry at the branch-gating item causes a rule error, including when that item is marked. | Test |
| FR-064-AC-9 | Empty, missing, non-directory, duplicate, overlapping or out-of-base roots; a selection with no production Rust files; unreadable or malformed source; malformed rules; and a source link escaping the base or forming a cycle cannot produce a clean result. Each returns a typed library error identifying its input and command exit 2. Rule failures include unknown or missing fields, wrong types, empty reasons and out-of-base entry paths. | Test |
| FR-064-AC-10 | Installing `qsl-string-edge` using the documented authoritative Git route and running it outside QSL against a non-QSL crate succeeds without QSL-named fixture directories. The documented clean fixture exits 0. Replacing its production body with an unmarked literal-string dispatch exits 1 and names the expected consumer file and line. A separate Rust consumer imports the public scanner API through the documented Git dependency without copied scanner sources. | Test |
| FR-064-AC-11 | QSL's adapter passes its roots and rules to the standalone library, with no second detector implementation. On clean, violating and invalid-input fixtures, its report content and success/failure agree with the library; its existing lint prerequisite still propagates failure. Extraction adds only the scanner crate's own tests to QSL verification. | Test |
| FR-064-AC-12 | A fixture function marked `#[ix_trace_rs::string_edge]`, one marked `#[::ix_trace_rs::string_edge]` and one marked with `#[string_edge]` imported by `use ix_trace_rs::string_edge;` each hold a string comparison that the library, the command and QSL's adapter do not report; an unmarked function in the same fixture is reported. | Test (TC-162) |
| FR-064-AC-13 | A consumer crate that applies `#[string_edge]` from `ix-trace-rs` to a function compiles and runs the function with the behaviour of the unmarked function; `#[string_edge(anything)]` fails to compile with the one error message `#[string_edge] takes no arguments` and no second error from the retained function. | Test (TC-162) |
| FR-064-AC-14 | The QSL workspace defines no attribute macro named `string_edge` and no re-export or alias of one, holds no crate whose only purpose is that marker, and `cargo check` of every package resolves each `string_edge` marker in production code to `ix_trace_rs::string_edge`. | Test (TC-162) |
| FR-064-AC-15 | A fixture function marked `#[my_macros::string_edge]` is reported, as are functions whose only mark is the comment `// string_edge` or the doc comment `/// string_edge`; and `cargo metadata` over the QSL workspace shows `ix-trace-rs` among the dependencies of every package that uses the marker. | Test (TC-162) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §9
  (string dispatch rule and the edge table, which cites ADR-010 §4.3's five
  production dispatch sites this requirement's allow-list rejection names).
- [FR-062](FR-062-implement-checked-family-contract.md).
- [US-006](../usecase/US-006-extend-a-family-without-breaking-seams.md).
- agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-codegen#86 run
  the same scan in their own repositories, over the same rule; this
  requirement builds the scan tool that #141 and #86 invoke, not their own
  edge inventories. The attribute is `ix-trace-rs`'s.
- Linear QSL-953 (`ix-trace-rs`: publish the `#[string_edge]` attribute with the
  contract in "The marker attribute") is the prerequisite for QSL's use of the
  attribute; `ix-trace-rs` provides `trace` and `implements` only until it lands.
