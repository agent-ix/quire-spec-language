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
choose behaviour. QSL SHALL provide a tool attribute that marks an edge
function and a scan that reports every string comparison or string `match`
outside a marked function in the QSL crates' non-test code, so this rule is
enforced by a running check rather than by convention alone.

## Inputs

- The QSL crates' source (non-test code; `#[cfg(test)]` modules and files
  under a `tests/` directory are out of scope for the scan).
- The `#[string_edge]` tool attribute applied to an edge function.
- A checked-in allow-list of string comparisons that compare a user-supplied
  value and select no behaviour on its content.

## Outputs

- An `xtask string-edge` report listing every string comparison or string
  `match` found outside a `#[string_edge]`-marked function and not covered by
  the allow-list, with the file and line of each.
- A non-zero `xtask string-edge` exit when that list is non-empty; a zero
  exit when it is empty.

## Behavior

### The marker attribute

`#[string_edge]` SHALL be a tool attribute applicable to a function. It
SHALL carry no runtime behaviour: it exists only for `xtask string-edge` to
read, and its presence or absence SHALL NOT change what the marked function
compiles to.

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
freshly computed digest's hex string against a cached one purely as an
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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-064-AC-1 | Given a QSL source tree with one string comparison inside a function marked `#[string_edge]` and one string `match` inside an unmarked function, `xtask string-edge` reports exactly the unmarked occurrence, naming its file and line, and does not report the marked occurrence. | Test (TC-162) |
| FR-064-AC-2 | Given an unmarked occurrence whose file and line matches an allow-list entry, `xtask string-edge` does not report it; removing that allow-list entry causes the same occurrence to be reported on the next scan, with no change to the source. | Test (TC-162) |
| FR-064-AC-3 | `xtask string-edge` scans no file under a `tests/` directory and no `#[cfg(test)]` module; a string `match` placed only inside such a module is not reported even when unmarked and not allow-listed. | Test (TC-162) |
| FR-064-AC-4 | `xtask string-edge` exits non-zero when its report is non-empty and exits zero when its report is empty; a test constructs one fixture tree of each shape and asserts both exit codes. | Test (TC-162) |
| FR-064-AC-5 | Given one allow-list entry whose comparison result feeds an `if`/`match` condition that selects between two different code paths, and one entry whose comparison result feeds only a logged message, `xtask string-edge` rejects the first (naming its file and line) and accepts the second. A test attempts to add each of the five ADR-010 §4.3 production dispatch sites (named in this requirement's Behavior) as an allow-list entry and asserts the tool rejects all five; a wrong implementation that allow-lists all five to force a clean scan does not satisfy this criterion. | Test (TC-162) |
| FR-064-AC-6 | The lint gate invokes `xtask string-edge`, and a test that stubs the gate's target list shows the gate fails when `xtask string-edge` exits non-zero. | Test (TC-162) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §9
  (string dispatch rule and the edge table, which cites ADR-010 §4.3's five
  production dispatch sites this requirement's allow-list rejection names).
- [FR-062](FR-062-implement-checked-family-contract.md).
- [US-006](../usecase/US-006-extend-a-family-without-breaking-seams.md).
- agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-codegen#86 run
  the same scan in their own repositories, over the same rule; this
  requirement builds the attribute and the scan tool that #141 and #86
  invoke, not their own edge inventories.

## Status

Specified under
[#214](https://github.com/agent-ix/quire-spec-language/issues/214). ADR-012
§9's edge table names the string-dispatch sites this scan is expected to
find clean or flag once QSL's own edges are marked; that marking work is
this ticket's and the family migration tickets' own, not this requirement's
scan tool.

**Scope of what #214 delivers.** `#[string_edge]` and `xtask string-edge`
are both fully implemented and tested (TC-162): the scan correctly finds
literal-based string comparisons and string-literal `match` arms outside a
marked function, respects the allow-list, rejects a branch-gating allow-list
entry (including all five ADR-010 §4.3 sites), and excludes test code.
Every edge in the files #214 itself touches or adds (`xtask/src/*`) is
marked and the scan is clean there. Running the scan against the whole QSL
crate as it stands today reports 60 further occurrences, all outside
`qsl-semantics/src/family/*` and `qsl-eval/src/value/expression/*` -- in `src/cli.rs`,
`src/complete/`, `qsl-semantics/src/model/`, `src/protocol_artifact/`, `src/linking.rs`,
`src/mapped.rs`, `src/package/intake.rs`, `src/state/evaluation.rs` and
`qsl-semantics/src/value/definition.rs`. Almost all are branch-gating, so the allow-list
(which this requirement's own Behavior section forbids from admitting a
branch-gating entry) cannot make them clean; converting them is real work
belonging to whichever family or module owns that code, not to #214's own
migration of function declaration/application. This is tracked as
[QSL-145](https://linear.app/agent-ix/issue/QSL-145), parented to #214's own
tracking issue, not attempted here.

Because of this, the "Gate placement" behavior above and FR-064-AC-6 are
**not** satisfied by production wiring in #214: `make string-edge` runs the
tool standalone (see the Makefile), not as part of `ci:`, until QSL-145
lands -- wiring it into `ci:` today would fail the gate on 60 sites #214
did not introduce and does not own, the same shape as FR-063's S2/S3
deferral (QSL-143). TC-162's own test (step 7, a stubbed gate target list)
still demonstrates the wiring *mechanism* works; it does not demonstrate
the real crate is clean under it.

**By Acceptance Criterion (PR #262 review, P3 accounting), with real trace
tags as they exist in the delivered code today:**
- FR-064-AC-1: backed (`TC-162`, `xtask/src/string_edge.rs`).
- FR-064-AC-2: unbacked (untagged). The scanning half is real (any occurrence
  the allow-list doesn't cover is reported); the specific
  add-then-remove-reappears sequence has no dedicated tagged test. Owner:
  QSL-150.
- FR-064-AC-3: backed (`TC-162`, `xtask/src/string_edge.rs`).
- FR-064-AC-4: unbacked (untagged). No test asserts the CLI's process exit
  code directly (the underlying `Result`/`Err` shape that drives it is
  exercised indirectly through other tagged tests). Owner: QSL-150.
- FR-064-AC-5: unbacked (untagged; PR #262 review, coordinator round 3,
  finding 7; previously misrecorded as backed). Its branch-gating/
  non-branching distinction half is exercised by two real tests in
  `xtask/src/string_edge.rs` (`branch_gating_is_distinguished_from_a_non_
  branching_sink`, `allow_list_entry_at_a_branch_gating_occurrence_is_
  rejected`), now untagged rather than left implying the whole criterion.
  Its "each of the five ADR-010 §4.3 production dispatch sites" half is
  not: `none_of_the_five_adr010_production_sites_can_be_allow_listed`
  builds five *synthetic* fixtures shaped like the five sites, but
  `branch_gating_entries` filters only on `(file, line, branch_gating)`,
  never on the literal string compared, so the test passes identically for
  five arbitrary branch-gating strings -- it never actually attempts to
  allow-list the five real, named sites at their real file:line. One of
  those five (the `"allocation"` relationship-category site, `QSL:model/
  systems.rs:269` per ADR-010 §4.3's own evidence column, which that same
  column already flags "PR-sensitive") is confirmed gone from
  `qsl-semantics/src/model/systems.rs` on this branch, so a real test against the
  current tree could not reject all five as currently named regardless.
  Owner: QSL-150.
- FR-064-AC-6: unbacked, as this section's own paragraph above already
  states at length (no production gate wiring in #214). Owner: QSL-145
  (the first half -- no production gate wiring); QSL-155, a spec defect,
  owns the second half.

Two of six ACs are backed (AC-1, AC-3); four are unbacked
(AC-2, AC-4, AC-5, AC-6).
