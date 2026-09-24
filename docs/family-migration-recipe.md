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
   family's `Observed` outcome, or the family's `Refused` reason. In the
   same change, reach it from S6a: add the family's `S6aFamilyKind` variant
   to the `s6a_family_kinds!` list in `qsl-eval/src/value/expression/s6a.rs` (which also puts
   it in `S6aFamilyKind::ALL`), its `S6aFamilyKind::family` arm, its
   `evaluate_declaration` arm in `qsl-eval/src/value/expression/mod.rs`, and its arm
   in TC-385's `s6a_family_name` match. `Relation` has no evaluator: S6a's
   input type has no `Relation` variant (ADR-012 §2, FR-090-AC-4).
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
contract for this reason (see `qsl-semantics/src/family/mod.rs`'s own module doc). A
family migration whose claim forms *do* carry a real FR-057 capability kind
adds `requirements()` back to the contract in that same change, with a real
non-`None` arm to justify it -- StateModel's model-element lookups and
Relation's refinement claims are the two families in ADR-012 §3's table
most likely to need this first.

## Removal condition

[ADR-011](../spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md)
§7.3's M-6e, in its own terms: **the family's old composed-checker path
(SEAM-2, `src/checking/composed/`) is deleted in the same pull request that
lands that family's S3 family checker and S4 emission.** Each migrated form
has exactly one S3 checker, the family's own check code (FR-065-CON-2 states
this for function declaration and application; every later migration
follows the same rule).

The rule is verified by behaviour, never by a symbol-absence, file-location
or code-shape test (testing-policy ruling, Peter, 2026-09-22): a migrated
form's own family check gives the same verdict from every entry point that
reaches it, nested forms are checked under that entry point's clause kind
(FR-065's Behavior section), and the family's tests assert what that verdict
admits and refuses.
FR-065-AC-4 and FR-065-AC-5 are the worked example. A dispatch arm that
calls the family's check holds no logic of its own (ADR-012 §4.3); that is
a design constraint verified by inspection (FR-065-CON-3).

Per [ADR-012](../spec/decisions/ADR-012-semantic-family-extension-contracts.md)
§14.1, at least one implementing ticket for each remaining family:

| Family | Implementing ticket(s) |
| --- | --- |
| `StateModel` | #120, #121, #164 (via #220) |
| `SumCase` | #187 (via #221) |
| `TemporalTrace` | #188, #189 (via #222), including the FR-300 mapping |
| `ProtocolClause` | #218 (via #223) |
| `Relation` | #191, #192, #198 (via #223) |
| Remaining `Value` forms (literals, operators, `let`, `if`, records, collections) | #120, #164, #170, #175; the last family ticket overall deletes the remainder of the composed checker (ADR-011 §7.3 M-6e) |

## Worked example: function declaration and application (#214/FR-065)

This ticket's own migration is the concrete instance of every category
above.

**Required tests, as delivered:**
- Clause-level unit: `qsl-eval/src/value/expression/family.rs`'s
  `family_contract_tests::value_function_family_checks_through_the_contract`,
  and `qsl-semantics/src/check/family.rs`'s
  `checking_tests::two_contexts_from_the_same_declarations_check_identically`.
- Builder-ordering: not applicable to this family -- a function declaration
  has no independently meaningful clause sequence (ADR-012 §4.1's test);
  this is recorded rather than silently skipped.
- Seam-probe: `xtask/src/seam_probe.rs`'s checked-in `checked_in_locations()`
  names the two real S1 seams this migration adds (`FamilyKind`'s
  `catalog_code_prefix` prefix arm and `stage_hooks`'s stage-participation
  table, both in `qsl-semantics/src/family/mod.rs`), demonstrated by a real failing build
  under `RUSTFLAGS=--cfg seam_probe` (FR-063).
- Wire-totality: `family_contract_tests::value_function_family_checks_through_the_contract`
  asserts the v2 round trip (`ValueFunctionFamily::package` then
  `decode_v2`) recovers the minted identity, and `qsl-semantics/src/family/mod.rs`'s own
  `FamilyKind` `match`es (the seam-probe targets above) have no `_` arm.
- Backend-absence corpus: not applicable -- function declaration and
  application requests no FR-057 capability kind (see this document's
  `requirements()` note above), so there is no backend-absence case to
  construct for this family; a family that does request a kind adds this
  test category for real.

**Required conversions, as delivered:**
- v2 emitter: `CheckedPackage::emit_function_package_v2` in
  `qsl-eval/src/value/expression/mod.rs`, calling `family::emit_v2`/`decode_v2`
  directly. `FamilyContract::package` (and `ValueFunctionFamily`'s
  implementation of it) is deleted (PR #262 review, findings F1/F2 -- see
  the entry below): `emit_function_package_v2` originally called it into a
  scratch buffer nothing read, then built its real returned bytes
  independently, so the hook had no consumer.
- Evaluator: `impl ReferenceEvaluation for ValueFunctionFamily` in
  `qsl-eval/src/value/expression/family.rs` (`evaluate`); the trait is layer 5's, in
  `qsl-eval/src/value/expression/s6a.rs` beside `S6aFamilyKind`, so the impl sits in
  the trait's crate (the orphan rule) while `ValueFunctionFamily` stays in
  layer-3 `check`. It is reached through the S6a
  seam `evaluate_declaration` in `qsl-eval/src/value/expression/mod.rs`, which
  `CheckedPackage::call` calls.
- Requirement derivation: deferred for this family, per this document's own
  note above.

**Real deleted symbols from this migration** (FR-066-AC-4). **Correction
(PR #262 review, finding F13):** an earlier revision of this section cited
`src/family/requirements.rs`, `PackageRefusal`,
`EvaluateRefusal::Incomplete`, `DeclarationCause` and `Stage::Requirements`
as this ticket's "real deleted symbols." Those five were deleted from an
uncommitted working draft before this ticket's first commit
(`7ec1302`) ever landed -- from a reader's point of view, indistinguishable
from a symbol that was never proposed at all, which is precisely the
"hypothetical placeholder" AC-4 forbids citing. `Requirements`,
`CapabilityKind`, `Extent`, `Bound` and `Stage::Requirements` are recorded
unbacked for AC-4 rather than cited as deleted (they are real deferrals --
this document's `requirements()` note above and `qsl-semantics/src/family/mod.rs`'s own
module doc explain why -- just not ones a git log entry can show being
removed). `PackageRefusal`, `EvaluateRefusal::Incomplete` and
`DeclarationCause` are dropped from this list entirely: `package` was
narrowed to take no `Result` (so it never needed a `PackageRefusal`) rather
than have one deleted, and `EvaluateRefusal`/function-declaration's `Cause`
were never given those variants in any committed revision to delete from.
The following, in contrast, existed in `7ec1302` (pushed, inspectable with
`git show 7ec1302` or later) and were removed by a later, also-pushed
commit on this branch, in the PR #262 review round -- real deletions
someone can find:
- `FamilyContract::package` and `ValueFunctionFamily`'s implementation of it
  (`qsl-semantics/src/family/contract.rs`, `src/value/expression/family.rs`) -- deleted
  because `emit_function_package_v2` never read its output (F1/F2 above).
- `stage_hooks`, `Stage` and `HookStatus` (`qsl-semantics/src/family/mod.rs`) -- deleted
  because their only non-test callers were three `assert_eq!` sites
  asserting a hand-written `match`'s own literal result against itself, and
  removing those fabricated callers left the table with no real reader
  (PR #262 review, finding F7; see `qsl-semantics/src/family/mod.rs`'s own module doc and
  FR-063's "Correction to merged spec" note on the seam-probe's checked-in
  list).
- `FamilyKind::all()` and `assert_distinct_catalog_code_prefixes`
  (`qsl-semantics/src/family/mod.rs`) -- deleted for the same reason as `stage_hooks`
  (F7); the compile-time distinctness check `const _` in the same file
  replaces the property it asserted with one enforced on every build, not
  only under `cargo test`.
- `StageFailure::Fault` and this crate's own `InternalFault`
  (`qsl-semantics/src/family/outcome.rs`) -- deleted (F7): their one construction site
  compared `mint_declaration_identity`'s output against itself, which
  cannot fail by construction, not by anything the runtime checked.
  `src/diagnostic.rs`'s own `InternalFault` (landed later, from `#213` S-5)
  is that type's real eventual home.
- The "defensive Fault path" self-comparison in
  `ValueFunctionFamily::check` (`src/value/expression/family.rs`) -- deleted
  along with `StageFailure::Fault` above, for the same reason.

**Real test file added:** `qsl-eval/src/value/expression/family.rs`'s
`family_contract_tests` module (added by this migration); `xtask/src/
seam_probe.rs` (new file, this migration's seam-probe implementation).

## What this recipe does not cover

FR-064 (`#[string_edge]`/`xtask string-edge`) and its own crate-wide marking
sweep are a separate, cross-cutting concern from this per-family recipe;
see [QSL-145](https://linear.app/agent-ix/issue/QSL-145) for the remaining
string-dispatch sites a family's own migration may need to mark or convert
as it touches that family's code.
