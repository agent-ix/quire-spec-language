---
id: TC-792
title: "StateModel causes raised under Value and ProtocolClause evaluation render with the state-model prefix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-301
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: verifies
---
# TC-792: StateModel causes raised under Value and ProtocolClause evaluation render with the state-model prefix

## Description

Verify that a model cause raised inside an enclosing family's evaluation keeps
`StateModel` attribution. Scope: FR-301-AC-1, FR-301-AC-2.

The typed Value-call controls additionally verify FR-100-AC-14 through
FR-100-AC-19 and FR-286-AC-7. They supplement the two original procedures
and results.

## Test Procedure

1. Compile a unit with a `Value` function whose body is `lookup<T>(p, r)`;
   run it through the spine run entry with a population in which `r` is
   absent and the lookup is `absent refused`.
2. Compile a unit with a state clause whose body calls a dispatched query
   operation whose effective precondition is `false` for the receiver; run
   the clause over a snapshot holding that receiver.

### Typed Value-call controls for step 1

Use the actual owning source parser, model selection/normalization and
population/object admission APIs. Do not construct checked nodes, a trusted
effective view, fixed opaque ids, or an evaluator-only substitute. The native
source declares the existing Population parameter form and a Reference
result; its body is exactly `lookup<T>(p, r) absent refused`. Keep the same
source and selected model for both controls. If the source/checked producer
is unavailable, retain this test UNRUN rather than count that failure as
the required model refusal.

Admit an object world holding a real object `r`, and a closed query
population in the same selected model/universe that does not contain it.
Use the PopulationId minted by real binding admission and registered in
that actual environment, and the world's admitted Reference. Call
`qsl_replay::spine::run` with shared typed arguments, the borrowed context,
the actual caller limits and the original Cancel. Retain evidence that
the lookup reached the enclosing Value evaluation: its real catalog cause,
payload and lookup location, not merely a generic non-success exit.

Repeat with an admitted population containing `r`, preserving the source,
model, universe and Reference triple. Make independent mutations of kind,
unregistered population identity, declared maximum, model/universe and
object-world closure. Include a function that ignores its arguments to
check that admission is independent of body use. Combine a wrong-kind
value with a later missing parameter to retain binding-failure precedence.
Exercise malformed raw identity input through the real constructor/reader;
do not forge a typed malformed reference.

### Worked Reference-admission vectors

Use a real source-selected model `M` with concrete object types `T`, `S`
and `U`, where `S` declares `T` as a supertype and `U` is unrelated to
`T`. Arrange an actual model connection so `U` shares `T`'s admitted object
universe without becoming its subtype; do not assign an invented universe.
Admit real objects `t:T`, `s:S` and `u:U` and their closure through the
owner APIs, deriving all effective identities from those declarations.
Admit a closed population of `T` including `t` and `s`, with the same
declared maximum as the source parameter. Compile the exact lookup body
with a key parameter declared `Reference<T>`. Thus the `s:S` input tests
call admission into a supertype parameter, not merely lookup with a
parameter already declared `Reference<S>`.

Independently admit a second actual selected model `M2`. Retain that model's
full admitted selection, types and universe instead of forging identities.
For the foreign-model
vector supply its genuinely admitted population binding/context to the
source compiled against `M`. Within `M`, also admit a concrete type `W`
and real object `w:W` in a separate actual object-universe component from
`T`. For the foreign-universe vectors use `w`'s real Reference while the
call retains `M`'s required population. This separates a foreign universe
from a foreign model selection. The combined vector explicitly establishes
that `W` also does not conform to `T`; it does not require forging an
independent type/universe pairing to manufacture a universe-only case.
Every deliberately foreign value remains well-formed under its real owner.

| Vector | Input to the compiled `Reference<T>` parameter / context | Required observation |
| --- | --- | --- |
| Exact | Real `t:T` in `M`'s admitted world and containing population | Admission succeeds; lookup completes with the exact original `t` Reference |
| Proper subtype | Real `s:S`, with the admitted `S -> T` supertype edge | Admission succeeds; lookup completes with the original most-specific `S` Reference, not a retagged `T` Reference |
| Unrelated in-model | Real `u:U`, same selected model and universe, no `U -> T` path | Call-stage `WrongValueKind` at the key's declared position; no lookup/evaluation |
| Foreign model | Actual `M2` binding/context against source selected to `M` | Owning `foreign_reference` / `foreign-model-selection`, preserving the actual offered-selection arm and full required selection; no conformance/evaluation |
| Foreign universe | Real `w:W` from `M`'s separate admitted universe with `M`'s required population | Owning `foreign_reference` / `foreign-universe`, actual required/supplied universe bytes; no conformance/evaluation |
| Combined | Real `w:W`, additionally showing the admitted graph has no `W -> T` path | Same actual `foreign-universe` cause and fields as the universe failure, never WrongValueKind or dangling; no conformance/evaluation |

Repeat all six vectors with a source-compiled Boolean function that declares
the same inputs and returns `true` without using them. Exact and subtype
inputs complete `true`; the four invalid vectors keep their admission
refusals and zero evaluation consumption despite unused parameters.
Independently confirm that the unchanged kernel exact predicate rejects
the proper subtype for `Reference<T>` while the real selected-model QSV
conformance predicate accepts it; this predicate comparison supplements,
and does not replace, the public-call execution controls.

Give two correctly named parameters independently invalid inputs in both
argument-list orders: after names are validated, the first failing declared
parameter wins, not input list order. Combine a wrong-shape value with a
later missing parameter as above: the missing-name refusal still wins.
The genuine subtype lookup also repeats the valid-world absent-population
control, retaining StateModel `invalid_runtime_input` / `absent-key` rather
than being intercepted by exact-type call admission.

Retain all existing scalar tests and their oracles when adapting callers to
the replacement signature. Exercise actual Requested and Deadline causes
at front-end, applicable admission and evaluation boundaries; retain their
different causes and the complete configured accounting limits. Check
zero evaluation work for pre-call failures and exact/one-less real lookup
evaluation charges with admission already permitted, without resetting or
borrowing a helper/population budget. No source-derived count is a runtime
measurement. Repeat the absent/present pair through the driver's use of the
same reviewed public Rust entry, without driver-owned carrier semantics.

For FR-286-AC-7, serialize the present run through the existing
`OutcomeDocument::from_run` route. Also compile a Reference-valued function
that returns its second admitted Reference argument while receiving a
distinct first argument. Both references come from the real admitted
world. Compare the typed result and function outcome document to the
actual second reference, not the first. This general returned-value control
does not replace either original procedure or the exact lookup body.

## Expected Results

1. `FamilyResult::Refused` with the actual StateModel-owned model-query
   refusal, not a Value-owned substitute; category refusal. Independently
   assert the unchanged `ModelRefusal::catalog_code()`: code exactly
   `invalid_runtime_input`, cause exactly `absent-key`, with the actual
   binding/key fields and lookup locus. A generic invalid-input refusal,
   an admission refusal or a fabricated `state-model` code prefix fails.
2. `FamilyResult::Undefined` with the actual StateModel-owned dispatched
   cause `precondition-false`, not a ProtocolClause-owned substitute;
   category undefined. Independently assert the owning undefined reason
   and the actual called operation, selected method, receiver and dispatched
   call locus. A generic undefined result or fallback cause fails.

The test's `state-model` prefix means cause-family attribution as FR-301
defines it; it is not string concatenation onto a catalog code. Retain
evidence of the owning typed cause and its projection into the enclosing
family result as well as the independent exact catalog/reason checks;
matching the rendered code alone does not establish StateModel ownership.

### Typed Value-call control results

- The absent control reaches the actual lookup and produces the enclosing
  `FamilyResult::Refused` with StateModel attribution, code exactly
  `invalid_runtime_input`, the owning `absent-key` cause, actual
  population/key fields and source location, category refusal, exit 20.
  Setup, compilation, result-kind selection and input admission refusals
  cannot satisfy it.
- The present control completes with the exact supplied Reference triple,
  category success, exit 0; result serialization retains its universe,
  most-specific type and object identity. Always refusing lookup fails.
- The separate returned-value control preserves the actual returned
  second Reference in FR-286's existing completed `result.value`, with no
  substitution by the first argument, object state or Population payload.
- Invalid input retains the owning typed construction, model or call
  admission refusal and required fields. A dangling reference is distinct
  from an admitted real reference absent only from the query population.
  No evaluation runs, including for unused parameters. Missing parameter
  takes precedence over the independent wrong-kind value.
- For each actual Admission cause in FR-100's command-error table, inspect
  the original typed payload and execute its command-error projection.
  Compare the closed details exactly: offered modelIdentity versus offered
  population DeclarationKey and full expected selection; ForeignType's
  actual member/type; ForeignUniverse's actual required/supplied bytes;
  dangling input's parameter position. Preserve each exact cause and
  original detail message. Inject an extra details member, remove a
  required member and substitute another cause independently: each must
  be rejected by the owning FR-267 strict reader. These ModelRefusal arms
  carry no locus; do not invent one. Real earlier located failures and
  evaluated lookup loci remain at their actual owning boundaries.
- Scalar results and all original refusal/internal-fault oracles remain
  equal. All ten actual accounting ceilings and each actual Cancel cause
  remain intact. Exact/one-less evaluation uses real charge records, not
  inferred helper totals; Requested and Deadline remain distinguishable.
- Driver and library observations agree on these outcomes through the
  same reviewed call boundary. Until actual source, shared-helper,
  strict-reader/admission and consumer qualification exist, these controls
  remain UNRUN. The separate step 2 dispatch/snapshot acceptance remains
  with its SM1 owner.
