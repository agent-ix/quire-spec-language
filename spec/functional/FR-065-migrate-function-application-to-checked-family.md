---
id: FR-065
title: "Migrate function declaration and application onto the checked-family contract"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
---
# FR-065: Migrate function declaration and application onto the checked-family contract

## Description

Function declaration and application is the sole representative family this
ticket migrates onto the checked-family contract of
[FR-062](FR-062-implement-checked-family-contract.md). QSL SHALL make the
`Value` family's function-declaration and function-application forms check,
package and evaluate exclusively through that contract, carry their identity
and provenance through the checked-package boundary unchanged, and reach
callers only through the checked-package producer spine, with the
function-application arms of `infer_form` reduced to a thin dispatch seam
(ADR-012 §4.3).

This requirement covers only function declaration and application. The
remaining `Value` forms (literals, operators, `let`, `if`, records,
collections) and every other family's forms are unchanged by this
requirement and migrate under their own tickets.

## Inputs

- Parsed function-declaration and function-application forms.
- The checked-package producer spine: S1 (CST) through S4 (linked checked
  package, and its v2 emission on request).
- The typed `QualifiedName` a caller uses to select a function.

## Outputs

- A checked function-declaration node and a checked function-application
  node, each carrying an identity minted once at check and reachable through
  the package's occurrence-keyed source map.
- `quire.checked-package/v2` bytes carrying the function's identity and
  source-map occurrences unchanged from the in-process checked package.
- For a caller that selects a function by `QualifiedName` against a checked
  package: the checked function-application node's evaluated outcome, or a
  typed refusal.

## Behavior

### Public API accepts only checked objects or checked-package bytes

The function packaging/lowering public API SHALL accept a checked node (built
only through the contract's `check` hook) or `quire.checked-package/v2` bytes
verified through the package boundary (ADR-011 E4/E5). It SHALL NOT accept
raw CST, raw source text or a token stream as an input from which it derives
function semantics.

### `infer_form`'s function-application arms are thin

The function-application arms of `infer_form` (ADR-012 §4.3; today
`value/expression/check.rs:683-966`) SHALL each make exactly one call into
the `Value` family's own check code, hold no semantic logic of their own, and
propagate the family's returned outcome without inspecting or branching on
its content beyond wrapping it in the seam's own enum variant. Building the
called function's argument, and propagating its result with `?`, is not
semantic logic; a branch, a lookup or a check performed directly in the arm
is, and SHALL NOT appear there after this migration.

Every other `infer_form` arm (the `Present` and `Value` option operations,
the `Deref` model-element-reference read, and the operation-postcondition
`Pre` anchor) is unchanged by this requirement; only the function-declaration
and function-application arms move.

### Identity and provenance survive checking and package conversion

A function declaration's checked identity, minted once at check from its
content-addressed preimage, SHALL be the same identity read from the checked
package after linking (S4) and, when v2 bytes are emitted, the same identity
decoded from the v2 wire node. A function-application node's checked identity
SHALL likewise survive linking and v2 emission unchanged.

Every source occurrence of a function declaration or a function-application
call, keyed by (identity, role, ordinal), SHALL resolve to the same source
span before and after the checked-package boundary: the occurrence-keyed
source map SHALL carry the occurrence through E3 and E4 without re-minting
any span.

### The checked-package producer spine is the only producer

Native `run` and `compile`, where they produce a checked package or a
backend artifact, SHALL do so only through the checked-package producer
spine (S1 through S4, and E4 v2 emission on request); they SHALL NOT produce
one through the native-v1 path. The `lower` command, `package::NativePackage`
and wire form `native-linked-package/1` are deleted by this requirement's
implementation, in the same change that lands the spine replacement for the
commands that produced them (ADR-011 §7.3 M-6a, T-1). `format` is retargeted
to operate over the CST (S1) rather than a native-v1 parse.

After this requirement's implementation, exactly one path produces a checked
function package or a backend artifact from `run` or `compile`: no caller,
CLI flag or library entry point selects the deleted native-v1 producer path,
because that path's code no longer exists. A caller that could previously
select it is refused with a compile error (the deleted types and command are
gone) or a documented CLI argument change, not a silent fallback.

### `format` is retargeted, not deleted

`format`, unlike `run`, `compile` and `lower`, is retargeted to consume the
CST (S1) directly rather than deleted, because formatting is a tooling
consumer of the lossless CST (ADR-011 §2.3, "Only to tooling"), not a
checked-package producer.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-065-CON-1 | This requirement's implementation modifies no internal representation of any family other than `Value`'s function-declaration and function-application forms, and no `infer_form` arm other than those two forms'. | Design | Inspection |
| FR-065-CON-2 | This requirement's implementation deletes native `run`/`compile` package and backend-artifact production, the `lower` command, `package::NativePackage` and `native-linked-package/1` in the same change that lands their checked-package-spine replacement; no change under this requirement leaves both the deleted path and its replacement selectable at once. | Process | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-065-AC-1 | Calling the function packaging/lowering public API with a checked node or verified checked-package bytes succeeds; a test that attempts to call it with a raw CST node or a raw source string fails to compile (no accepting overload or conversion exists), not merely at runtime. | Test (TC-163) |
| FR-065-AC-2 | Given a source file declaring one function and one call to it, the function declaration's checked identity, read after S4 linking and again after decoding emitted v2 bytes, is the same value in all three readings; reordering unrelated top-level declarations in the source leaves that identity unchanged. | Test (TC-163) |
| FR-065-AC-3 | Given the same source file, the call's source occurrence (identity, role, ordinal) resolves to the same byte span before linking, after linking, and after decoding from v2 bytes; corrupting one byte of the occurrence's region in a hand-built alternate package makes the resolved span differ, showing the test actually reads the region rather than a constant. | Test (TC-163) |
| FR-065-AC-4 | The `infer_form` function-declaration and function-application arms each contain exactly one call into `Value`'s family check code and no other conditional, lookup or loop; a code-shape test (an AST or line-count check against a fixed budget) fails if a future change reintroduces branching logic directly in either arm. | Test (TC-163) |
| FR-065-AC-5 | After the implementation lands, the repository contains no `package::NativePackage` type, no `lower` command, and no reader or writer for wire form `native-linked-package/1`; a grep-equivalent test over the compiled crate's public symbols and the CLI's command table confirms their absence. `run` and `compile` invoked to produce a package or backend artifact route only through the S1-S4 spine, verified by a test that instruments the spine's entry function and asserts it is called at least once and the deleted native-v1 producer function (absent) is never called. | Test (TC-164) |
| FR-065-AC-6 | `format` invoked on a source file succeeds using only the CST (S1) as its input, verified by a test that supplies a source file whose native-v1 parse would fail (a construct only the deleted native-v1 parser rejected) but whose CST is well-formed, and observes `format` succeed. | Test (TC-164) |

## Dependencies

- [FR-062](FR-062-implement-checked-family-contract.md) defines the contract
  this migration implements for the `Value` family's function forms.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1 (E3/E4 admitted types and provenance), §2.3 (refusals, limits, partial
  output), §7.3 (M-6a and M-6e lane deletions, T-1 ticket row) own the spine
  edges and the deletion timing this requirement follows.
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §4.3
  (thin dispatch seam), §14.2 (states that only the function-application
  arms of `infer_form` are in scope for this ticket).
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-04 (checked node identity), O-07 (source occurrence identity), O-11
  (qualified names), O-12 (source locations and provenance), O-15
  (typestate) own the canonical types this requirement's identity and
  provenance guarantees are built from.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).

## Status

Specified under
[#214](https://github.com/agent-ix/quire-spec-language/issues/214). Not yet
implemented. State/model, sum/case, temporal/trace, protocol/frame and
refinement migrations, and the remaining `Value` forms, are out of scope and
are tracked by their own tickets (#120, #121, #164, #170, #175, #187, #188,
#189, #191, #192, #198, #218 via #220-#223), per ADR-012 §14.1.
