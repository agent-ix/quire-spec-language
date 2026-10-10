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


Detector scope: a comparison is branch-gating when it feeds an
`if`/`while` condition or `match` scrutinee/guard, is a term of a `&&`/`||`
chain, is a match arm's own value, or is a `strip_prefix` call.
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

- The QSL crates' source (non-test code; `#[cfg(test)]` items, files declared
  by `#[cfg(test)] mod x;` and files under a `tests/` directory are out of
  scope for the scan; a file is never out of scope by its name alone).
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
| FR-064-AC-6 | The lint gate's own target list (the `Makefile`) names `string-edge` as a prerequisite of the gate it runs in, whose own recipe runs `cargo xtask string-edge`, and `xtask string-edge`'s non-zero exit propagates to that gate's own exit through `make`'s ordinary prerequisite-failure semantics -- a `.PHONY` aggregate target depending on a target whose recipe can fail. Both halves are checked directly: a grep-shaped check over the real `Makefile` confirms the prerequisite naming and the recipe, and a minimal fixture `Makefile` of the same aggregate/prerequisite shape demonstrates a failed prerequisite failing the aggregate target (and a succeeding one not failing it). | Test (TC-162) |

The Behavior clause asks for a report on any comparison between a
`&str`/`String` value and *any other* such value, and any `match` whose
scrutinee is a string. The detector covers a literal and a same-crate named
constant on one side; it does not resolve a comparison between two bindings
(for example `adapter.observation_contract_revision != offered
.observation_contract_revision`, neither side a literal or a named
constant) or a constant defined in another crate. Both shapes are
undetected today, not merely unmarked -- the scan cannot see them to report
them. Extending the detector to those shapes is unticketed follow-up work,
not scope this requirement claims to close.

`qsl-semantics/src/check/claims.rs`'s `identity == NARROW`/`SCALAR_FAMILIES`
compares (SR-758 FND-005) are marked `#[string_edge]`
through one `operation_role` classifier, corrected here from an earlier
claim that this was a typed conversion: `operation_role` re-derives the
classification from the identity string on every call; nothing resolves it
once and threads a stored value. Mutation-testing the two checks that read
it (`claims.rs`'s narrow lookup and `emit.rs`'s `forced_absent` `EnumValue`
arm, SR-758 FND-006) found neither is distinguished by any fixture in the
tree today; both are kept as defensive checks against a future shape that
would make the distinction load-bearing, documented at each site rather
than removed, since FR-322 states the underlying rule as a property of the
node kind, not as a fact this tree's current inputs happen to make trivial.

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

