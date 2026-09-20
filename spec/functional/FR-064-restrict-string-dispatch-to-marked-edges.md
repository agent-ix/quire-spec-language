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
user-supplied value (for example, an error message the user chose to log,
compared for a test's own assertion) that selects no program behaviour based
on its content. An entry that gates a branch, a lookup or a dispatch
decision on the compared string's value SHALL NOT be added to the allow-list;
such an occurrence is refused by making its function an edge, marking it
`#[string_edge]`, and converting the string once, or by moving the dispatch
to a closed enum or typed identity.

### Gate placement

`xtask string-edge` SHALL run in the lint gate.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-064-AC-1 | Given a QSL source tree with one string comparison inside a function marked `#[string_edge]` and one string `match` inside an unmarked function, `xtask string-edge` reports exactly the unmarked occurrence, naming its file and line, and does not report the marked occurrence. | Test (TC-162) |
| FR-064-AC-2 | Given an unmarked occurrence whose file and line matches an allow-list entry, `xtask string-edge` does not report it; removing that allow-list entry causes the same occurrence to be reported on the next scan, with no change to the source. | Test (TC-162) |
| FR-064-AC-3 | `xtask string-edge` scans no file under a `tests/` directory and no `#[cfg(test)]` module; a string `match` placed only inside such a module is not reported even when unmarked and not allow-listed. | Test (TC-162) |
| FR-064-AC-4 | `xtask string-edge` exits non-zero when its report is non-empty and exits zero when its report is empty; a test constructs one fixture tree of each shape and asserts both exit codes. | Test (TC-162) |
| FR-064-AC-5 | Applying `#[string_edge]` to a function changes no compiled behaviour: a test compiles the function with and without the attribute and asserts the generated behaviour (via its observable outputs, not its assembly) is identical. | Test (TC-162) |
| FR-064-AC-6 | The lint gate invokes `xtask string-edge`, and a test that stubs the gate's target list shows the gate fails when `xtask string-edge` exits non-zero. | Test (TC-162) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §9
  (string dispatch rule and the edge table).
- [FR-062](FR-062-implement-checked-family-contract.md).
- [US-006](../usecase/US-006-extend-a-family-without-breaking-seams.md).
- agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-codegen#86 run
  the same scan in their own repositories, over the same rule; this
  requirement builds the attribute and the scan tool that #141 and #86
  invoke, not their own edge inventories.

## Status

Specified under
[#214](https://github.com/agent-ix/quire-spec-language/issues/214). Not yet
implemented. ADR-012 §9's edge table names the string-dispatch sites this
scan is expected to find clean or flag once QSL's own edges are marked; that
marking work is this ticket's and the family migration tickets' own, not
this requirement's scan tool.
