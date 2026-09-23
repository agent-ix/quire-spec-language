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
declaration and application; it does not perform the CLI cutover. Spine
`compile` and spine `run`, the deletion of native `compile`, the `lower`
command and `lowering`, and the retarget of `format` to the CST are
ADR-011 §7.3 M-6a, T-1's own PR, carried
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

### The contract's `check` hook makes the typing verdict

The `Value` family's `check` hook for a function declaration SHALL type-check
the declaration's body against its declared result type. It SHALL refuse an
ill-typed declaration through the contract's refusal outcome
(`StageFailure::Refused`), carrying an `ill_typed` cause, and SHALL record no
success diagnostic for it. It SHALL admit a well-typed declaration with the
declaration's minted identity.

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

**Correction: the identity preimage is a pragmatic stopgap, not the
external schema (PR #262 review, finding F10).** This requirement's
implementation mints each identity as a SHA-256 over the package identity
and the declaration's or call's own **parsed** structure, rendered through
Rust's `Debug` formatting (`{:?}`) -- name, parameters, result, measure and
body for a declaration; callee name and parsed arguments for a call -- not
the checked/typed tree, so the hash is independent of unrelated
declarations' reordering (AC-2). This is a pragmatic content-address for
this ticket's own scope, not a claim of interop with the external
`quire.checked-package-id/v2` `ApplicationNode`/`PreimageTerm` schema
(`resources/complete-value/.../node-identity-preimage.schema.json`): that
schema's `Operation` identity for an arbitrary applied operator has no
landed implementation this ticket could follow for a user-declared
function, and a `Debug`-rendered preimage is not that schema's own
preimage format. Structural identity within one check run -- reordering
independence and survival across S4 linking and v2 emission, what AC-2 and
AC-3 actually test -- holds regardless of this gap. Building the real
`PreimageTerm`-conformant preimage is separate work; QSL-156 owns it.

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

Where the composed checker module is retained for those other forms, its
dispatch `match` over its own input form-kind enum SHALL carry no `_` or
catch-all arm, the same rule FR-063 applies at every S1 to S4 seam. This
enum is the composed checker's own construct, not one of FR-063's S1-S4
enums, so FR-063's probe does not reach it; this requirement states the
same no-wildcard rule directly, for this one enum, so the same defect it
prevents elsewhere cannot reopen here. Because no catch-all arm is admitted,
deleting the function-declaration and function-application match arms is a
compile error (`E0004`) for as long as the composed checker's input
form-kind enum still carries a function-declaration or
function-application variant. This requirement's implementation SHALL
therefore also remove those two variants from that enum in the same
change, which is what makes the arm deletion compile at all: no `_ =>
refuse(...)` arm may be added to keep the match exhaustive instead, because
that would be a catch-all arm and is forbidden by this same rule. A function
form is thereafter not merely unmatched by the composed checker; it is not
a value the composed checker's input type can hold.

### This requirement's checked-package producer is #240's precondition

This requirement builds S1 through S4 (and E4 v2 emission on request) for
function declaration and application: a checked-package producer that
`run`, `compile` and a future backend-artifact caller can route through.
This requirement does not add spine `compile` or spine `run`, does not
delete native `compile`, the `lower` command or `lowering`, and does not
retarget `format`. Those changes are ADR-011 §7.3 M-6a, T-1's own PR
(#240), which deletes each old path in the same change that lands its spine
replacement, per the T-3 same-change rule; #240 cannot land that PR before
this requirement's producer, and #242's S4 emitter, both exist. Native `run`
(native-run/1 clause execution), `package::NativePackage` and the
native-linked-package/1 reader stay until ADR-011 §7.3 M-6c (ADR-011-OQ-2).

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
| FR-065-AC-5 | After the implementation lands, the composed linker's pre-migration function-declaration and function-application checking entry points are absent from the compiled crate's symbols; a grep-equivalent test over the compiled crate's public and crate-internal symbols confirms their absence. Where the composed checker module is retained for its other `Value` forms, its input form-kind enum carries neither a function-declaration nor a function-application variant, and its dispatch `match` carries no `_` or catch-all arm; a test that reintroduces either variant into that enum without adding a matching arm fails to compile with `E0004`, and a test that instead adds a `_ => refuse(...)` arm to keep the match exhaustive while the variant stays fails this criterion, because a catch-all arm is disallowed by this requirement's own rule, not merely discouraged. A change that lands the S3 function checker while leaving either variant in the composed checker's input enum, with or without an arm for it, does not satisfy this criterion. | Test (TC-164) |
| FR-065-AC-6 | The layer-6 `replay` facade's executor entry, given a replay request naming a function, resolves the function by a typed `QualifiedName` against the recompiled package's declarations; a test that attempts to call the entry point with a bare `&str` in place of a `QualifiedName` fails to compile, and a request naming an unresolvable `QualifiedName` returns a typed refusal rather than matching by display-name equality. | Test (TC-166) |
| FR-065-AC-7 | Checking the declaration `g() -> Boolean = 1` (an `Integer` body against a declared `Boolean` result) through the `Value` family's contract `check` hook returns `StageFailure::Refused` whose cause is `ill_typed` / `type-mismatch`, and the diagnostic sink holds no entry afterwards. Checking the well-typed declaration `f() -> Boolean = true` through the same hook returns the checked declaration, whose identity equals the identity minted for `f`. | Test (TC-380) |

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
[#214](https://github.com/agent-ix/quire-spec-language/issues/214). State/model,
sum/case, temporal/trace, protocol/frame and refinement migrations, and the
remaining `Value` forms, are out of scope and are tracked by their own
tickets (#120, #121, #164, #170, #175, #187, #188, #189, #191, #192, #198,
#218 via #220-#223), per ADR-012 §14.1. The M-6a CLI producer cutover
(spine `compile` and `run`, deletion of native `compile`, `lower` and
`lowering`, and the `format` retarget) is
[#240](https://github.com/agent-ix/quire-spec-language/issues/240)'s own
requirement, owner-ruled against this ticket's contradiction between an
earlier draft of this requirement, QSL-25's body and ADR-011 T-1: this
requirement supplies #240's precondition (the checked-package producer) and
does not perform the cutover itself.

**This deferral is a citation, not a #214-local ruling (PR #262 review,
finding F1).** `src/cli.rs`'s `lower` command is still present after this
ticket lands; that is correct. ADR-011 §7.3's M-6a row
(`spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md`, the
"M-6a checked-package producer" line) names the scope it belongs to and
states its owner verbatim: "QSL-8 (this repo's #240) with M-4, before
#216". `src/package.rs`'s `NativePackageRef` belongs to native `run`, which
§7.3 retires in M-6c (ADR-011-OQ-2). Deleting `lower` inside #214 would contradict that
row, not satisfy it; #240 deletes it in the same change that lands spine
`compile` and `run` over this requirement's spine
(the T-3 same-change rule the Description section above already states),
which cannot happen before this requirement's own producer, and #242's S4
emitter, both exist.

**Scope of what #214 delivers, and what stays unbacked (PR #262 review
headline finding; rescoping decision recorded against #262, not an
amendment to this requirement's own target design above -- see the
correction notes under "Identity and provenance survive checking and
package conversion" and on `infer_form`'s/`Self::call`'s doc in
`check.rs`).** This requirement's own text -- "check... exclusively through
that contract" -- is the correct target design and is not narrowed here.
What #214 actually delivers against it: identity and provenance for both
forms mint exclusively through the checked-family contract
(`mint_declaration_identity`/`mint_call_identity`, reached only from
`ValueFunctionFamily::check` for declarations and from `Self::call` for
applications) and nowhere else -- that part is real. The checking decision
is split. A declaration's typing and definedness verdict is made inside
`ValueFunctionFamily::check` (QSL-148, PR #303; AC-7). A call is checked by
`check::family::check_application`, which `infer_form`'s `Call` arm calls
directly, not through a `FamilyContract` method. Termination is a separate
whole-package pass (`check::termination::check`) after every declaration is
checked.

By Acceptance Criterion, with real trace tags as they exist in the
delivered code today:
- FR-065-AC-1: unbacked. No `#[trace(..., "FR-065-AC-1")]` tag exists.
  Owner: QSL-154.
- FR-065-AC-2: backed (`TC-163`): `identity_survives_v2_round_trip`
  (`src/value/expression/family.rs`) and
  `function_identity_survives_reordering_check_linking_and_a_v2_round_trip`
  (`tests/dispatch_calls.rs`). Identity/provenance minting is the part this
  ticket actually delivers. **Rebuilt (PR #262 review, coordinator round
  3, finding 2).** The reordering half was previously backed by
  `identity_ignores_unrelated_declarations`
  (`src/value/expression/family.rs`), which minted the same identity twice
  from the same declaration and compared it to itself -- no second
  declaration was ever constructed, so the criterion's "does not depend on
  any other declaration's existence or position" clause had nothing to be
  independent of. It is deleted; the reordering clause is now backed by a
  real fixture at the `PackageDeclarations::check` level (two functions,
  checked in both orders, `CheckedPackage::function_identity` compared
  across both) in `tests/dispatch_calls.rs`, which also gives real test
  callers to `CheckedPackage::occurrence`, `emit_function_package_v2` and
  `decode_function_package_v2` (PR #262 review, coordinator round 3,
  finding 3).
- FR-065-AC-3: unbacked. No `#[trace(..., "FR-065-AC-3")]` tag exists
  anywhere in the crate (the one test that exercised it,
  `occurrence_span_survives_link_and_a_corrupted_alternate_differs`, was
  deleted as self-corrupting in the PR #262 review round, finding F6, and
  not replaced). Owner: QSL-154.
- FR-065-AC-4: **true by inspection, not backed** (PR #303 review, finding
  3). The fact AC-4 states is true of the delivered code:
  `infer_form`'s `Call` arm is exactly one call into
  `super::family::check_application` (`Value`'s function family's own
  checking code, `qsl-semantics/src/check/family.rs`) and holds no other conditional,
  lookup or loop. But AC-4 is itself a code-shape criterion ("a code-shape
  test... fails if a future change reintroduces branching logic directly in
  either arm"), and no test in the delivered code checks that shape:
  `check_application_accepts_a_well_typed_call`,
  `check_application_refuses_wrong_arity`,
  `check_application_refuses_an_unknown_name` and
  `check_application_refuses_a_type_mismatched_argument`
  (`#[trace("TC-376")]`, untagged for this criterion -- PR #303 review,
  finding N2) verify `check_application`'s own
  accept/refuse behavior, per the testing-policy ruling recorded on
  [QSL-148's Linear thread](https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837)
  (Peter, 2026-09-22, relayed by the QSL team lead: test what the family
  check accepts and refuses, not the arm's code shape) -- but a future
  change that reintroduces a conditional directly into `infer_form`'s `Call`
  arm, while leaving `check_application` itself unchanged, would still pass
  every one of those tests. TC-163's step 6 (the actual code-shape/AST
  check) is not implemented as a test either (see TC-163's own Status
  section) -- so nothing in the delivered code would catch that regression,
  and "backed" overstates what these tests demonstrate.
- FR-065-AC-5: **unbacked** (PR #303 review, finding 2; reverted from an
  earlier "backed" claim in this round). AC-5 requires that, once this
  requirement lands, "the composed checker's input form-kind enum carries
  neither a function-declaration nor a function-application variant," and
  states plainly that a variant left in place "with or without an arm for
  it, does not satisfy this criterion." `Expression::Call` is that variant,
  and it is still present in `Expression` (`qsl-forms/src/`) -- checking moved
  (`Typer::call` is deleted, and `check_declaration_body`/
  `check_application`, `qsl-semantics/src/check/family.rs`, are the real entry points
  `check::family::ValueFunctionFamily::check` now calls internally), but
  the variant itself was never removed, and QSL-148 does not add or claim a
  removal. `check_declaration_body_accepts_a_well_typed_declaration_and_
  reports_its_calls`, `check_declaration_body_refuses_an_ill_typed_body` and
  `check_declaration_body_refuses_an_undefined_body`
  (`#[trace("TC-377")]`, untagged for this criterion -- PR #303 review,
  finding N2) verify that entry point's real
  accept/refuse behavior and its call-reporting, per the same
  [testing-policy ruling](https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837)
  -- real, valuable coverage of the checking-decision half of this
  migration -- but they verify behavior, not AC-5's own enum-shape
  condition, and that condition is not met. See `TC-164`'s own Status
  section. Termination checking is explicitly excluded from this move --
  see `check_declaration_body`'s doc comment (`qsl-semantics/src/check/family.rs`) and
  this ticket's report for why a whole-package, cross-declaration
  call-graph analysis cannot become a per-declaration `FamilyContract`-style
  check; `check::termination::check` remains an unchanged, separate
  whole-package pass.
- FR-065-AC-6: unbacked. No `#[trace(..., "FR-065-AC-6")]` tag exists,
  though `CheckedPackage::call`'s typed-`QualifiedName` lookup
  (`src/value/expression/mod.rs`) is implemented; `TC-166` has zero tests
  in the delivered code (see FR-065's own Test Matrix / TC-166). Owner:
  QSL-5 / #243 -- a real owner that existed before this round but was not
  written against this criterion; recorded here now.
- FR-065-AC-7: backed (`TC-380`). Since QSL-148 (PR #303) the contract's
  `check` hook type-checks the body.
  `value_function_family_check_refuses_an_ill_typed_body` (refusal half,
  `qsl-semantics/src/check/family.rs` `checking_tests`) and
  `value_function_family_checks_through_the_contract` (admission half,
  `src/value/expression/family.rs`), are both tagged
  `#[trace("TC-380", "FR-065-AC-7")]`.

Two of this requirement's seven Acceptance Criteria are backed (AC-2,
identity/provenance; AC-7, the contract `check` hook's typing verdict); AC-4
is true by inspection but not backed by a test that could catch its own
regression; the other four (AC-1, AC-3, AC-5, AC-6) are unbacked, for the reasons above. `TC-165` and `TC-166` -- the
migration-recipe completeness check and FR-065-AC-6 -- have zero tests each
in the delivered code. `TC-164` keeps its own zero-test procedure (see its
Status section); the criterion it targets, AC-5, stays unbacked (see the
AC-5 row above), though `TC-377`'s real accept/refuse tests are genuine
coverage of the checking-decision half of this migration.
