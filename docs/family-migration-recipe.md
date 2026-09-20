# Semantic-family migration recipe

FR-066 (QSL#214). This document is the checked-in procedure a later ticket
follows to migrate one more semantic family (`StateModel`, `SumCase`,
`TemporalTrace`, `ProtocolClause`, `Relation`, or one of `Value`'s remaining
forms) onto the checked-family contract that
[FR-062](../spec/functional/FR-062-implement-checked-family-contract.md)
defines. It names no new runtime type, trait or module of its own: the
contract those steps apply is FR-062's, and the deletion timing they follow
is [ADR-011](../spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md)
§7.3's. It cites
[FR-065](../spec/functional/FR-065-migrate-function-application-to-checked-family.md)
(function declaration and application) as its worked example, because that
is the one family this ticket (#214) itself migrated.

## Required tests

[ADR-012](../spec/decisions/ADR-012-semantic-family-extension-contracts.md)
§3's "Tests" row names five categories every family migration needs. A
review checklist that omits any one of these does not satisfy FR-066-AC-1.

1. **Clause-level unit tests.** One test per clause or form the family
   parses, checking that clause in isolation against its own typing and
   validation rules -- not only as part of a larger fixture that happens to
   exercise it.
2. **Builder-ordering tests**, where the family's construct has
   independently meaningful clauses (ADR-012 §4.1): a test that submits the
   construct's clauses out of order and asserts the builder's typed
   clause-order refusal, and a test that submits them in order and asserts
   admission.
3. **The family's seam-probe coverage** (FR-063, ADR-012 §5.1): the
   family's own closed enums and `match` sites that ADR-012 §5.1 lists for
   S1-S4 carry a probe variant behind the `seam_probe` cfg, and the
   checked-in seam-function list (`xtask/src/seam_probe.rs`) names every one
   of them.
4. **Wire-totality tests** for the family's checked-node and `Cause` enums:
   a test that every real variant of the checked-node enum has an arm in
   the v2 emitter, and that every real variant of the family's `Cause` enum
   has a catalog code in `catalog_code()`, with no `_` arm making either
   `match` vacuously exhaustive.
5. **One backend-absence corpus case per capability kind** the family's
   claim forms request (ADR-012 §7.3): a fixture that requests a capability
   kind with no backend registered for it, asserting the typed
   backend-absence outcome rather than a panic or a silent no-op.

## Required conversions

FR-066-AC-2's three categories, per family:

1. **The v2 emitter**: the family's checked-node variant's conversion into
   `quire.checked-package/v2` bytes (ADR-013 O-17).
2. **The evaluator**: the family's `evaluate` hook (`ReferenceEvaluation`),
   converting a checked node plus its typing-context `Env` into the
   family's `Observed` outcome, or the family's `Refused` reason (or, for
   `Relation`, the explicit `FamilyOutcome::Refused(FamilyRefusal::
   FamilyNotNativelyEvaluable)` non-evaluability arm ADR-012 §3 names).
3. **Requirement derivation**: the family's pure `requirements()` function
   from a checked node to zero or one `Requirements` value (FR-062-AC-4);
   see this document's own note below on when this function has anything
   real to derive.

For a family whose forms cross into IR, RT or CG (`StateModel`, `SumCase`,
`TemporalTrace`, `ProtocolClause`, `Relation` all do, per ADR-012 §8's stage
table), the wire and IR-side conversions belong to those repositories' own
tickets, not to the QSL migration ticket: ADR-012 §14.1 names
`agent-ix/quire-contract-ir#141` (IR tag/form enums at v2 intake) and
`agent-ix/quire-contract-codegen#86` (CG enum matches) as the owners of
that side, and an RT ticket the RT owner opens separately.

**A note on `requirements()`.** #214's own implementation defers
`Requirements` and its substructure (`CapabilityKind`, `Extent`, `Bound`)
entirely: FR-057/#229 (QSL-11, Done) states plainly that "family-body
admission is language admission, not a capability kind" and that there is
"no kind for an expression nested in a clause, such as a function
application" (FR-057:159-186). Function declaration and application, the
family this ticket migrates, has no FR-057 capability kind at all, so its
`requirements()` would return `None` unconditionally -- a function with
exactly one always-taken branch is not a real implementation of a pure
function from checked node to requirement, it is the function's own
absence wearing a signature. #214 does not add `requirements()` to the
contract for this reason (see `src/family/mod.rs`'s own module doc). A
family migration whose claim forms *do* carry a real FR-057 capability kind
adds `requirements()` back to the contract in that same change, with a real
non-`None` arm to justify it -- StateModel's model-element lookups and
Relation's refinement claims are the two families in ADR-012 §3's table
most likely to need this first.

## Removal condition

[ADR-011](../spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md)
§7.3's M-6e, in its own terms: **the family's old composed-checker path is
deleted in the same pull request that lands that family's S3 family checker
and S4 emission.** At no point does a change land where both the old path
and the checked-family contract's replacement are reachable for the same
family at once (FR-065-CON-2 states this for function declaration and
application specifically; it is the general rule every later migration
follows). Where the composed checker module is retained for other forms of
the same family that have not yet migrated, its own dispatch `match` over
its input form-kind enum carries no `_` or catch-all arm (the same rule
FR-063 applies at S1-S4, stated directly for this one enum because FR-063's
probe does not reach it -- it is the composed checker's own construct, not
one of the four S1-S4 enums). This is what makes the arm deletion a compile
error until the corresponding variant is also removed from the input
enum: a `_ => refuse(...)` arm restores exhaustiveness without deleting the
variant, and is itself a forbidden catch-all, so it does not satisfy the
removal condition either.

Per [ADR-012](../spec/decisions/ADR-012-semantic-family-extension-contracts.md)
§14.1, at least one implementing ticket for each remaining family:

| Family | Implementing ticket(s) |
| --- | --- |
| `StateModel` | #120, #121, #164 (via #220) |
| `SumCase` | #187 (via #221) |
| `TemporalTrace` | #188, #189 (via #222), including the FR-300 mapping |
| `ProtocolClause` | #218 (via #223) |
| `Relation` | #191, #192, #198 (via #223) |
| Remaining `Value` forms (literals, operators, `let`, `if`, records, collections) | #120, #164, #170, #175 -- the last of which removes the composed checker module entirely |

## Worked example: function declaration and application (#214/FR-065)

This ticket's own migration is the concrete instance of every category
above.

**Required tests, as delivered:**
- Clause-level unit: `src/value/expression/family.rs`'s
  `family_contract_tests::value_function_family_checks_through_the_contract`
  and `two_contexts_from_the_same_declarations_check_identically`.
- Builder-ordering: not applicable to this family -- a function declaration
  has no independently meaningful clause sequence (ADR-012 §4.1's test);
  this is recorded rather than silently skipped.
- Seam-probe: `xtask/src/seam_probe.rs`'s checked-in `checked_in_locations()`
  names the two real S1 seams this migration adds (`FamilyKind`'s
  `catalog_code_prefix` prefix arm and `stage_hooks`'s stage-participation
  table, both in `src/family/mod.rs`), demonstrated by a real failing build
  under `RUSTFLAGS=--cfg seam_probe` (FR-063).
- Wire-totality: `family_contract_tests::value_function_family_checks_through_the_contract`
  asserts the v2 round trip (`ValueFunctionFamily::package` then
  `decode_v2`) recovers the minted identity, and `src/family/mod.rs`'s own
  `FamilyKind` `match`es (the seam-probe targets above) have no `_` arm.
- Backend-absence corpus: not applicable -- function declaration and
  application requests no FR-057 capability kind (see this document's
  `requirements()` note above), so there is no backend-absence case to
  construct for this family; a family that does request a kind adds this
  test category for real.

**Required conversions, as delivered:**
- v2 emitter: `ValueFunctionFamily::package` in
  `src/value/expression/family.rs`, and `CheckedPackage::
  emit_function_package_v2` in `src/value/expression/mod.rs`.
- Evaluator: `impl ReferenceEvaluation for ValueFunctionFamily` in
  `src/value/expression/family.rs` (`evaluate`), reached from
  `CheckedPackage::call` in `src/value/expression/mod.rs`.
- Requirement derivation: deferred for this family, per this document's own
  note above.

**Real deleted symbols from this migration** (FR-066-AC-4): the contract's
own shape narrowed as this ticket discovered which parts had no real
construction for a family with no capability kind and no typed cause yet
(recorded in `src/family/mod.rs`'s and `src/family/contract.rs`'s module
docs, and in ADR-012 §14.1's own record of this deferral):
- `src/family/requirements.rs` -- deleted in its entirety (the `Requirements`
  type and its `CapabilityKind`/`Extent`/`Bound` substructure).
- `PackageRefusal` (was in `src/family/contract.rs`) -- deleted; `package`
  narrowed to `fn package(checked: &Self::Checked, out: &mut Vec<u8>)`
  with no `Result`.
- `EvaluateRefusal::Incomplete` -- deleted (kept only `Refused(String)`);
  `quire-exact`'s own `Meter::charge`/`charge_plan` are `pub(crate)`
  (`quire-exact/src/accounting.rs:551,595`), not exported, so no family
  migrated so far can construct a real `Incomplete`.
- `DeclarationCause` and its `catalog_code()` method (was in
  `src/value/expression/family.rs`) -- deleted; function declaration has no
  real typed refusal cause distinct from the checking refusals `Value`
  already has, so a probe over it would test only its own mapping, not a
  seam.
- `Stage::Requirements` (was in `src/family/mod.rs`) -- deleted along with
  `Requirements` above.

**Real test file added:** `src/value/expression/family.rs`'s
`family_contract_tests` module (added by this migration); `xtask/src/
seam_probe.rs` (new file, this migration's seam-probe implementation).

## What this recipe does not cover

FR-064 (`#[string_edge]`/`xtask string-edge`) and its own crate-wide marking
sweep are a separate, cross-cutting concern from this per-family recipe;
see [QSL-145](https://linear.app/agent-ix/issue/QSL-145) for the remaining
string-dispatch sites a family's own migration may need to mark or convert
as it touches that family's code.
