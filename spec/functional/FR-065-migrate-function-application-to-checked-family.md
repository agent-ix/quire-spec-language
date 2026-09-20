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
package and evaluate exclusively through that contract, delete the composed
linker's function-declaration and function-application checking code in the
same change (ADR-011 §7.3 M-6e), carry their identity and provenance through
the checked-package boundary unchanged, and reach callers only through the
checked-package producer this requirement builds, with the
function-application arms of `infer_form` reduced to a thin dispatch seam
(ADR-012 §4.3).

This requirement builds the checked-package producer for function
declaration and application; it does not perform the CLI cutover. Native
`run` and `compile`'s deletion of their native-v1 producer path, the `lower`
command, `package::NativePackage`, `native-linked-package/1`, and the
retarget of `format` to the CST are ADR-011 §7.3 M-6a, T-1's own PR, carried
by [#240](https://github.com/agent-ix/quire-spec-language/issues/240)
because that cutover cannot land before #242's S4 emitter exists. This
requirement's checked-package producer is #240's precondition, not its
implementation.

This requirement covers only function declaration and application. The
remaining `Value` forms (literals, operators, `let`, `if`, records,
collections) and every other family's forms are unchanged by this
requirement and migrate under their own tickets, and the composed linker's
checking code for those remaining forms is unchanged by this requirement.

## Inputs

- Parsed function-declaration and function-application forms.
- The checked-package producer this requirement builds: S1 (CST) through S4
  (linked checked package, and its v2 emission on request).
- The typed `QualifiedName` a caller uses to select a function, including
  the layer-6 `replay` facade's executor entry (ADR-012 §9's edge table;
  today the bare `&str` function-name lookup at
  `value/expression/mod.rs:635`).

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

### The composed function checker is deleted in this change

The composed linker's (SEAM-2) checking code for function declaration and
function application SHALL be deleted in the same change that lands this
requirement's S3 family checker and S4 emission for those two forms (ADR-011
§7.3 M-6e). After this requirement's implementation, no composed-checker
code path checks a function-declaration or function-application form; the
checked-family contract's `check`/`package` hooks
([FR-062](FR-062-implement-checked-family-contract.md)) are the only path
that does. The composed checker's remaining `Value` forms (literals,
operators, `let`, `if`, records, collections) are unchanged by this
requirement and are deleted by their own migrating tickets (#120, #164,
#170, #175), the last of which removes the composed checker module entirely
(ADR-011 §7.3 M-6e).

### This requirement's checked-package producer is #240's precondition

This requirement builds S1 through S4 (and E4 v2 emission on request) for
function declaration and application: a checked-package producer that
`run`, `compile` and a future backend-artifact caller can route through.
This requirement does not wire `run` or `compile` onto that producer, does
not delete the native-v1 producer path, the `lower` command,
`package::NativePackage` or `native-linked-package/1`, and does not
retarget `format`. Those five changes are ADR-011 §7.3 M-6a, T-1's own PR
(#240), which deletes each old path in the same change that lands its spine
replacement, per the T-3 same-change rule; #240 cannot land that PR before
this requirement's producer, and #242's S4 emitter, both exist.

### The replay executor selects a function by typed name

The layer-6 `replay` facade's executor entry, which selects the function to
call for a replay request, SHALL take a typed `QualifiedName` (ADR-013 O-11)
and SHALL resolve it against the recompiled package's declarations
(ADR-012 §8, §9). It SHALL NOT take a bare `&str` compared against a
function's display name; the function-name lookup by `&str` at
`value/expression/mod.rs:635` is replaced by this typed lookup as part of
this requirement's implementation, widening #243's layer-6 `replay` facade
for the function family.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-065-CON-1 | This requirement's implementation modifies no internal representation of any family other than `Value`'s function-declaration and function-application forms, and no `infer_form` arm other than those two forms'. | Design | Inspection |
| FR-065-CON-2 | This requirement's implementation deletes the composed linker's function-declaration and function-application checking code in the same change that lands the S3 family checker and S4 emission for those two forms; no change under this requirement leaves both the composed checker's function-form arms and the checked-family contract's function checker reachable at once. | Process | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-065-AC-1 | Calling the function packaging/lowering public API with a checked node or verified checked-package bytes succeeds; a test that attempts to call it with a raw CST node or a raw source string fails to compile (no accepting overload or conversion exists), not merely at runtime. | Test (TC-163) |
| FR-065-AC-2 | Given a source file declaring one function and one call to it, the function declaration's checked identity is the same value read at three points: immediately after `check`, again after S4 linking, and again after decoding the emitted v2 bytes. Reordering unrelated top-level declarations in the source leaves that identity unchanged at all three points. | Test (TC-163) |
| FR-065-AC-3 | Given the same source file, the call's source occurrence (identity, role, ordinal) resolves to the same byte span before linking, after linking, and after decoding from v2 bytes; corrupting one byte of the occurrence's region in a hand-built alternate package makes the resolved span differ, showing the test actually reads the region rather than a constant. | Test (TC-163) |
| FR-065-AC-4 | The `infer_form` function-declaration and function-application arms each contain exactly one call into `Value`'s family check code and no other conditional, lookup or loop; a code-shape test (an AST or line-count check against a fixed budget) fails if a future change reintroduces branching logic directly in either arm. | Test (TC-163) |
| FR-065-AC-5 | After the implementation lands, the composed linker's function-declaration and function-application checking entry points are absent from the compiled crate's symbols (or, where the composed checker module is retained for its other `Value` forms, its function-form match arms are removed so calling it with a function form fails to compile); a grep-equivalent test over the compiled crate's public and crate-internal symbols confirms their absence. A change that lands the S3 function checker while leaving the composed checker's function-form arms reachable, even if no caller currently invokes them, does not satisfy this criterion. | Test (TC-164) |
| FR-065-AC-6 | The layer-6 `replay` facade's executor entry, given a replay request naming a function, resolves the function by a typed `QualifiedName` against the recompiled package's declarations; a test that attempts to call the entry point with a bare `&str` in place of a `QualifiedName` fails to compile, and a request naming an unresolvable `QualifiedName` returns a typed refusal rather than matching by display-name equality. | Test (TC-166) |

## Dependencies

- [FR-062](FR-062-implement-checked-family-contract.md) defines the contract
  this migration implements for the `Value` family's function forms.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1 (E3/E4 admitted types and provenance), §2.3 (refusals, limits, partial
  output, and the "Only to tooling" CST-consumer row `format` falls under
  once #240 retargets it), §7.3 owns the deletion timing this requirement
  follows: M-6e (the composed function checker, deleted by this
  requirement) and M-6a, T-1 (the CLI producer cutover, deleted by
  [#240](https://github.com/agent-ix/quire-spec-language/issues/240) against
  the producer this requirement builds, after
  [#242](https://github.com/agent-ix/quire-spec-language/issues/242)'s S4
  emitter lands).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §4.3
  (thin dispatch seam), §8 (the replay stage hook this requirement widens
  for the function family), §9 (the replay-executor edge this requirement
  converts to a typed `QualifiedName`), §14.2 (states that only the
  function-application arms of `infer_form` are in scope for this ticket).
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
#189, #191, #192, #198, #218 via #220-#223), per ADR-012 §14.1. The M-6a CLI
producer cutover (native `run`/`compile` deletion, `lower`,
`package::NativePackage`, `native-linked-package/1`, and the `format`
retarget) is [#240](https://github.com/agent-ix/quire-spec-language/issues/240)'s
own requirement, owner-ruled against this ticket's contradiction between an
earlier draft of this requirement, QSL-25's body and ADR-011 T-1: this
requirement supplies #240's precondition (the checked-package producer) and
does not perform the cutover itself.
