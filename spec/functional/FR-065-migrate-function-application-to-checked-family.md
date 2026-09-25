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
[FR-062](FR-062-implement-checked-family-contract.md).
QSL SHALL check, package and evaluate the `Value` family's
function-declaration and function-application forms exclusively through that
contract and `Value`'s own family check code (Behavior: the contract's
`check` hook and `Value`'s application check).
QSL SHALL carry their identity and provenance through the checked-package
boundary unchanged (Behavior: identity and provenance).
QSL SHALL expose these forms to callers only through the checked-package
producer this requirement builds (Behavior: the public API and #240's
precondition). Each
form has one S3 checker (FR-065-CON-2), and the S3 typer's `Call` arm is a
thin dispatch seam into `Value`'s application check (FR-065-CON-3, ADR-012
§4.3).

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
requirement and migrate under their own tickets.

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

### `Value`'s application check decides every function application

`Value`'s function-application check is `check::family::Application`
(`qsl-semantics/src/check/family.rs`: `resolve`, `parameter`, `finish`).
When the S3 typer checks a call `name(arguments)`:

- If `name` names a declared function callable by name, `Application` SHALL
  resolve the call to that function.
- If the argument count differs from that function's parameter count,
  `Application` SHALL refuse the call as `ill_typed`/`type-mismatch`.
- `Application` SHALL supply each argument's expected type from that
  function's declared parameter types.
- `Application` SHALL build the call node with that function's declared
  result type.
- If `name` names a declared tuple type, `Application` SHALL resolve the call
  to that type's constructor, SHALL refuse an argument count that differs
  from the tuple's position count as `ill_typed`/`type-mismatch`, and SHALL
  build a tuple node of that type.
- If `name` names a declared type that is not a tuple, or a model operation,
  `Application` SHALL refuse the call as `ill_typed`/`operator-ineligible`.
- If `name` names no callable function, no type and no model operation,
  `Application` SHALL refuse the call as `missing-name` naming `name`.

The S3 typer (`Typer`, `qsl-semantics/src/check/check/typing.rs`) SHALL
check every `Expression::Call` it reaches through `Application`: in a
function declaration's body or `decreases` measure, which
`PackageDeclarations::check` reaches through `ValueFunctionFamily::check`,
and in a clause expression checked through
`CheckedGraph::check_clause_expression`. `Application`'s own verdict on a
call (its callee, its arity and its parameter types) is therefore the same
from every entry point. The typer SHALL check each argument against the
expected type `Application` supplies, under the entry point's clause kind,
so an argument whose admission depends on the clause kind (a dispatch call,
FR-151; `pre(...)`, FR-153) is admitted or refused as that clause kind
allows.

`Typer::infer_form`'s `Call` arm is a dispatch seam (ADR-012 §4.3): it makes
one call into `Application::resolve` and holds no name, arity or
argument-type logic of its own (FR-065-CON-3). `Expression::Call` is a
variant of the one `Expression` enum in the `forms` core (`qsl-forms`). Its
owning family is `Value`, because its arm calls `Value`'s application check
(ADR-012 §4.3). A call nests inside every other `Value` form, so
`Expression` carries the `Call` variant alongside them.

Every other `infer_form` arm (the `Present` and `Value` option operations,
the `Deref` model-element-reference read, and the operation-postcondition
`Pre` anchor) is unchanged by this requirement; only the `Call` arm is in
scope. A function declaration is not an expression and has no `infer_form`
arm.

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

Each identity is an ADR-013 O-04 checked node id: the
`quire.checked-semantic-node/v1` digest of the RFC 8785 encoding of the
node's preimage. A function declaration's node lists its parameter nodes and
references its body's root expression node, so its body holds no
application. It is keyed by QSL's `quire.structural-node/v1` preimage, which
names its `SourceOwner` (O-04;
[FR-092](FR-092-key-type-parameter-and-declared-nodes.md)). A call is its own
`expression` node, keyed by QSpec FR-322's `quire.application-node/v1`
preimage, which has no owner member because an expression carries no
declaration ([FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)).
Neither preimage names a `package_id` or any unrelated declaration, so the
identity does not depend on their order (AC-2). The declaration's identity
equals FR-092's golden vector, and the call's equals the FR-322
application-node key of the same node (AC-8). QSL-156 slice A4b switches the
checker's minter to these preimages.

### Each function form has one S3 checker

`ValueFunctionFamily::check` makes a function declaration's typing and
static-definedness verdict (FR-065-AC-7), and `PackageDeclarations::check`
reaches that verdict through it. `Value`'s application check above makes a
call's resolution and arity verdict. FR-065-CON-2 states that each is the
one S3 code path for its form.

The composed checker (SEAM-2, `src/checking/composed/`) checks the composed
grammar (ADR-010 lane B) and takes no `qsl-forms` input. Its `predicate`
declarations and `ValueKind::Invoke` calls are composed-grammar forms.
ADR-011 §7.3 M-6e deletes SEAM-2 one family at a time, each in the PR that
lands that family's S3 checker and S4 emission; the `Value` tickets are
#214, #120, #164, #170 and #175, and the last family ticket overall deletes
the remainder. Which `Value` ticket deletes SEAM-2's `predicate` and
`Invoke` checking is FR-065-OQ-1.

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
| FR-065-CON-1 | This requirement's implementation modifies no internal representation of any family other than `Value`'s function-declaration and function-application forms, and no `infer_form` arm other than `Expression::Call`'s. | Design | Inspection |
| FR-065-CON-2 | `ValueFunctionFamily::check` is the one S3 code path that types a function declaration and checks its static definedness, and `check::family::Application` is the one S3 code path that resolves a call's callee, checks its arity and supplies its argument types. Each change under this requirement leaves exactly one S3 path for each form. | Design | Inspection |
| FR-065-CON-3 | `Typer::infer_form`'s `Call` arm makes one call into `Application::resolve` and hands the result to the typer's argument descent; it holds no branch, lookup or check of its own (ADR-012 §4.3). Building the call's argument, wrapping its result in the typer's frame and propagating an error with `?` are not semantic logic. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-065-AC-1 | Calling the function packaging/lowering public API with a checked node or verified checked-package bytes succeeds; a test that attempts to call it with a raw CST node or a raw source string fails to compile (no accepting overload or conversion exists), not merely at runtime. | Test (TC-163) |
| FR-065-AC-2 | Given a source file declaring one function and one call to it, the function declaration's checked identity is the same value read at three points: immediately after `check`, again after S4 linking, and again after decoding the emitted v2 bytes. Reordering unrelated top-level declarations in the source leaves that identity unchanged at all three points. | Test (TC-163) |
| FR-065-AC-3 | Given the same source file, the call's source occurrence (identity, role, ordinal) resolves to the same byte span before linking, after linking, and after decoding from v2 bytes; corrupting one byte of the occurrence's region in a hand-built alternate package makes the resolved span differ, showing the test actually reads the region rather than a constant. | Test (TC-163) |
| FR-065-AC-4 | Given a declared function `f(x: Boolean): Boolean`, checking a call through the S3 typer admits `f(true)` as a call node of type `Boolean` whose callee is `f`; refuses `f(true, false)` (one argument too many) as `ill_typed`/`type-mismatch`; refuses `nowhere()` (a name that resolves to no function and no type) as `missing-name` naming `nowhere`; and refuses `f(1)` (an `Integer` argument against the declared `Boolean` parameter) as `ill_typed`/`type-mismatch`. | Test (TC-376) |
| FR-065-AC-5 | Given a package that declares `f(x: Boolean): Boolean`, each of the calls `f(true)`, `f(true, false)`, `nowhere(true)` and `f(1)` receives the same verdict as the body of a parameterless `Boolean`-result declaration checked through `PackageDeclarations::check` and as a precondition clause checked through `CheckedGraph::check_clause_expression`: `f(true)` is admitted in both as a `Boolean` call to `f`; `f(true, false)` and `f(1)` are refused `ill_typed`/`type-mismatch` in both; `nowhere(true)` is refused `missing-name` naming `nowhere` in both. Written as the `decreases` measure of a declaration whose body is `true`, `f(true, false)` and `f(1)` are refused `ill_typed`/`type-mismatch` and `nowhere(true)` is refused `missing-name`. Verdicts are compared by refusal cause, not location. | Test (TC-164) |
| FR-065-AC-6 | The layer-6 `replay` facade's executor entry, given a replay request naming a function, resolves the function by a typed `QualifiedName` against the recompiled package's declarations; a test that attempts to call the entry point with a bare `&str` in place of a `QualifiedName` fails to compile, and a request naming an unresolvable `QualifiedName` returns a typed refusal rather than matching by display-name equality. | Test (TC-166) |
| FR-065-AC-7 | Checking the declaration `g() -> Boolean = 1` (an `Integer` body against a declared `Boolean` result) through the `Value` family's contract `check` hook returns `StageFailure::Refused` whose cause is `ill_typed` / `type-mismatch`, and the diagnostic sink holds no entry afterwards. Checking the well-typed declaration `f() -> Boolean = true` through the same hook admits it. `PackageDeclarations::check` keys `f` once every declaration is typed, and under source owner (`a`, `u`) its checked identity is FR-092 vector F1, the `quire.structural-node/v1` key of its function node. | Test (TC-380) |
| FR-065-AC-8 | Under source owner (`a`, `u`), the checked identity of `function both using v(a: Boolean, b: Boolean): Boolean pure { a and b }` is FR-092 vector F2, a `quire.structural-node/v1` key whose preimage carries that owner, and the checked identity of the call `both(a, true)` in `function nb using v(a: Boolean): Boolean pure { both(a, true) }` is FR-092 vector E2, a `quire.application-node/v1` key whose preimage has no `owner` member. The application-node key builder reproduces every `operation_vectors` digest in QSpec's `node-identity-vectors.json`. | Test (TC-163) |

## Dependencies

- [FR-062](FR-062-implement-checked-family-contract.md) defines the contract
  this migration implements for the `Value` family's function forms.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1 (E3/E4 admitted types and provenance), §2.3 (refusals, limits, partial
  output, and the "Only to tooling" CST-consumer row `format` falls under
  once #240 retargets it), §7.3 owns the deletion timing this requirement
  follows: M-6e (the composed checker, SEAM-2, deleted one family at a
  time, the last family ticket deleting the remainder; FR-065-OQ-1) and M-6a, T-1 (the CLI producer cutover, deleted by
  [#240](https://github.com/agent-ix/quire-spec-language/issues/240) against
  the producer this requirement builds, after
  [#242](https://github.com/agent-ix/quire-spec-language/issues/242)'s S4
  emitter lands).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §4.3
  (thin dispatch seam; the one `Expression` enum, each variant owned by the
  family whose check its arm calls), §8 (the replay stage hook this requirement widens
  for the function family), §9 (the replay-executor edge this requirement
  converts to a typed `QualifiedName`), §14.2 (states that only the
  function-application arms of `infer_form` are in scope for this ticket).
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-04 (checked node identity), O-07 (source occurrence identity), O-11
  (qualified names), O-12 (source locations and provenance), O-15
  (typestate) own the canonical types this requirement's identity and
  provenance guarantees are built from.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md) and
  [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md) define
  the node shapes and preimages the declaration and call identities hash.
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

**What is delivered.** A declaration's and a call's identities are minted
by `check::lowering` keying after `PackageDeclarations::check` has typed
every declaration (FR-092, FR-093; QSL-156 A4b). A declaration's typing and
definedness verdict is made inside `ValueFunctionFamily::check` (QSL-148,
PR #303; AC-7). A call is checked by `check::family::Application`, which the
typer's `Call` arm calls for every call it reaches (AC-4, AC-5). Termination
is a separate whole-package pass (`check::termination::check`) after every
declaration is checked.

By Acceptance Criterion, with real trace tags as they exist in the
delivered code today:
- FR-065-AC-1: backed (`TC-163`, QSL-154). The function packaging/lowering
  public API's sole checked-node entry, `CheckedPackage::link`, accepts
  only a `CheckedGraph` -- built solely through the contract's `check`
  hook -- and a paired `compile_fail`/`no_run` doctest on `CheckedPackage::
  link` (`qsl-package/src/checked.rs`) demonstrates a raw CST node
  (`qsl_cst::ParsedSource`) and a raw source string (`String`) both fail to
  compile against it (`E0277`, no `Into<CheckedPackage>`), while a real
  checked node built through `check` and linked compiles and succeeds.
- FR-065-AC-2: backed (`TC-163`): `identity_survives_v2_round_trip`
  (`qsl-eval/src/value/expression/family.rs`) and
  `function_identity_survives_reordering_check_linking_and_a_v2_round_trip`
  (`qsl-eval/tests/it/dispatch_calls.rs`). Identity/provenance minting is the part this
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
  across both) in `qsl-eval/tests/it/dispatch_calls.rs`, which also gives real test
  callers to `CheckedPackage::occurrence`, `emit_function_package_v2` and
  `decode_function_package_v2` (PR #262 review, coordinator round 3,
  finding 3).
- FR-065-AC-3: backed (`TC-163`, QSL-154):
  `occurrence_span_survives_link_a_v2_round_trip_and_a_corrupted_alternate_differs`
  (`qsl-eval/tests/it/dispatch_calls.rs`) checks a real source file whose
  `f` calls `g`, resolves the call's own region immediately after `check`
  and again after `CheckedPackage::link`, resolves `f`'s own declaration
  region again once its identity has travelled a real `emit_function_
  package_v2`/`decode_function_package_v2` round trip (the v2 checkpoint
  is against the declaration's identity, the one thing this minimal v2
  encoding actually carries -- see the test's own doc for why the call's
  region has no v2 checkpoint to exercise), and confirms a hand-built
  alternate package whose `DeclarationSpans` is genuinely one byte wider
  resolves to a different region. Mutation-verified: temporarily made
  `region.rs`'s resolver return a constant span for every `Origin::Body`
  location, and confirmed this test (not merely `region.rs`'s own tests)
  caught it.
- FR-065-AC-4: backed (`TC-376`). Amended by QSL-148's spec lane to a
  behavioural criterion; the thin-arm rule it used to test by code shape is
  FR-065-CON-3, verified by inspection, per the
  [testing-policy ruling](https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837)
  (Peter, 2026-09-22). `check_application_accepts_a_well_typed_call`,
  `check_application_refuses_wrong_arity`,
  `check_application_refuses_an_unknown_name` and
  `check_application_refuses_a_type_mismatched_argument`
  (`qsl-semantics/src/check/family.rs`, `checking_tests`, tagged
  `#[trace("TC-376", "FR-065-AC-4")]`) each check an `Expression::Call`
  through `Typer::infer`, so each fails when `Application`'s resolution,
  arity check or parameter typing is removed.
- FR-065-AC-5: unbacked; a test is needed (`TC-164`). The criterion is
  amended by QSL-148's spec lane: it requires the same call verdict from a
  declaration body checked through `PackageDeclarations::check` and from a
  precondition clause checked through `CheckedGraph::check_clause_expression`,
  in place of the earlier symbol-absence and enum-variant conditions, which
  ADR-012 §4.3 contradicts (`Expression::Call` is a `Value`-owned variant of
  the one `Expression` enum) and the testing policy does not admit.
  `synthesized_dispatch_candidate_is_not_callable_by_name`
  (`qsl-eval/tests/it/dispatch_calls.rs`) shows a declaration body's
  `missing-name` refusal through `PackageDeclarations::check`, but no test
  compares the two positions.
- FR-065-AC-6: unbacked. No `#[trace(..., "FR-065-AC-6")]` tag exists,
  though `CheckedPackage::call`'s typed-`QualifiedName` lookup
  (`qsl-eval/src/value/expression/mod.rs`) is implemented; `TC-166` has zero tests
  in the delivered code (see FR-065's own Test Matrix / TC-166). Owner:
  QSL-5 / #243 -- a real owner that existed before this round but was not
  written against this criterion; recorded here now.
- FR-065-AC-7: backed (`TC-380`). Since QSL-148 (PR #303) the contract's
  `check` hook type-checks the body, and on the QSL-156 A4b branch, pending
  merge, `f`'s checked identity is FR-092 vector F1.
  `value_function_family_check_refuses_an_ill_typed_body` (refusal half,
  `qsl-semantics/src/check/family.rs` `checking_tests`) and
  `value_function_family_checks_through_the_contract` (admission half and
  F1, `qsl-eval/src/value/expression/family.rs`) are both tagged
  `#[trace("TC-380", "FR-065-AC-7")]`.
- FR-065-AC-8: backed on the QSL-156 A4b branch, pending merge (`TC-163`):
  `check` keys each function by FR-092 and each call by FR-093, `both` keys
  to F2 and `both(a, true)` to E2
  (`qsl-semantics/src/check/lowering/tests.rs`), and the application-node
  key builder in `qsl-semantics/src/check/node_key/` reproduces QSpec's
  operation vectors under the opt-in `make conformance`.

Six of this requirement's eight Acceptance Criteria are backed (AC-1, the
packaging API's checked-node-only entry; AC-2, identity/provenance; AC-3,
occurrence-span survival across check, linking and a v2 round trip; AC-4,
the application check's verdicts; AC-7, the contract `check` hook's typing
verdict and F1; AC-8, on the A4b branch); the other two (AC-5, AC-6) are
unbacked, for the reasons above.
`TC-164`, `TC-165` and `TC-166` have zero tests each in the delivered code.

## Open Questions

- **FR-065-OQ-1: Which `Value` ticket deletes SEAM-2's `predicate` and
  `Invoke` checking?** ADR-011 §7.3 M-6e deletes each composed family in the
  PR that lands its S3 family checker and S4 emission, and lists #214 among
  the `Value` tickets. SEAM-2's `predicate` declarations and `ValueKind::Invoke`
  calls are composed-grammar forms that reach no S3 checker today, and ADR-011
  §7.3's M-6a row records the 2026-09-19 owner ruling that nothing working is
  removed early. The answer names
  the ticket that deletes them together with the composed grammar's path to
  S3.
