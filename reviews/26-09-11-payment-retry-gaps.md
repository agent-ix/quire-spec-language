---
id: SR-378
title: "Gap analysis of observed repeat guards and the payment-retry emission leg"
type: SpecReview
analysis: gap-analysis
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md; docs/compiled-protocol-v1.md; src/protocol_artifact/native/families.rs; src/protocol_artifact/native/families/decisions.rs; src/protocol_artifact/native/families/decisions/received.rs; tests/native_payment_retry_emission.rs; tests/native_choice_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` examined `bb30eac..7e89134` on
`agent-a/payment-retry-repetition`: the FR-042 amendment admitting observed
(dynamic) Boolean repeat guards, its compiler implementation, and the two test
files that exercise it.

Plan completion is not applicable. `plan/` holds Plan-001 through Plan-009 and
no protocol-artifact-emission bundle, so there is no targeted plan to
reconcile (SR-355/SR-359 precedent).

The scoped tests pass: `tests/native_choice_emission.rs` 22/22 and
`tests/native_payment_retry_emission.rs` 9/9 under
`cargo test --jobs 1 -- --test-threads=1`. `quire coverage --scope . --json`
(quire 0.31.0, engine `ca7362d4`) reports 374/383 backed targets, zero
`status_lies`, six raw unbacked rows and three unmatched `IT-004` tags, all
inherited and outside this diff.

Every trace tag introduced by this diff binds. The specific defect looked for —
more than one `TC-` id inside a single `#[trace(...)]` attribute — does not
occur anywhere in the two scoped files; each new attribute cites exactly one
`TC-121` plus `FR-042-AC-*` ids that all resolve in `spec/`.

One high gap remains: the amendment's attempt-identity sentences promise
per-occurrence attempt identities carrying an iteration ordinal, and neither
the compiler nor the tests deliver that.

## Verdict

**FAIL** — FND-001 is a high finding: an amended FR-042 sentence and the
matching FR-042-AC-6 clause have no implementing code and no verifying test,
and the scoped test asserts the contrary cardinality. The inherited corpus rows
of FND-005 independently fail the strict matrix rule. The observed-repeat-guard
capability itself (FND-002 aside) is implemented, owned by requirement text and
backed by passing tests; nothing in this verdict says the guard work is wrong.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The amended per-occurrence attempt identity and its iteration ordinal are specified but not implemented and not tested; the scoped test asserts one static attempt identity instead. | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:106-108; FR-042-AC-6; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:145-149; src/protocol_artifact/wire.rs:489; tests/native_payment_retry_emission.rs:906 | correct-requirement-no-evidence |
| FND-002 | medium | Ten matrix rows understate their evidence: TC-121 and FR-042-AC-1..AC-9 are backed by passing tagged tests yet all read Planned. Recorded, not repaired — the matrix is owned by another branch. | spec/model-linking/tests.md:116,207-216 | wrong-requirement |
| FND-003 | medium | The observed-repeat partition is `guard` against `not guard`, so the completeness axis of the partition proof cannot fail; the admitted proof carries visibility and ownership only. FR-042's partition sentence is satisfied but vacuous on that axis. | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:169-172; src/protocol_artifact/native/families/decisions.rs | wrong-requirement |
| FND-004 | low | FR-042 says a body-established guard atom refuses "before family admission", but `native::admit` still surfaces `Unsupported::FamilyProof`, so a consumer cannot tell the two refusal classes apart from the admit result alone. | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:144-150; tests/native_choice_emission.rs:1744-1801 | wrong-requirement |
| FND-005 | low | Inherited corpus debt outside this diff: four unbacked rows (TC-115, FR-036-AC-5/-AC-6/-AC-8), two no-source-symbol exemptions (TC-010 Manual, FR-017-AC-2 Inspection), 20 untracked NFR-007 symbols and three unmatched IT-004 tags. | spec/model-linking/tests.md:112; spec/tests.md:45; tests/fixture_audit.rs | correct-requirement-no-evidence |

### FND-001 — the attempt-ordinal sentence has no implementation

Commit `d62ee1b` added to FR-042:

> The compiler SHALL retain one attempt identity per bounded occurrence of a
> forward attempt authored inside a bounded repeat, carrying its enclosing
> iteration ordinal.

and to FR-042-AC-6:

> A forward attempt repeated across bounded iterations retains one attempt
> identity per occurrence with its iteration ordinal ...

TC-121 step 5 repeats the obligation. Neither the compiler nor the wire
contract supplies it. The emitted `Attempt` event record is
`{owner, operation, contracts, instance}`; `instance` is the role instance, not
an iteration ordinal, and no record in the protocol artifact carries a repeat
occurrence index. The source diff of this branch touches only
`families.rs`, `decisions.rs` and `decisions/received.rs`, none of which mint
attempt identities.

The scoped test states the implemented behaviour plainly:

    assert_eq!(attempt.len(), 1, "one attempt identity, not one per bound");

The test file header also names "bounded occurrences A1/A2", while the authored
fixture declares only `attempt A1`; no `A2` exists in the source under test.

The remaining three amended attempt sentences are implemented and verified:
attempt distinct from the carrying delivery and from the declared effect, no
further effect minted on repetition, and no inherited earlier effect
(`tests/native_payment_retry_emission.rs`, the 1/2/1 send/delivery/effect
assertions and `effect.requires == vec![attempt_index]`).

Either the compiler must emit a per-occurrence attempt identity with its
ordinal, or the amended sentence and the AC-6 clause must be narrowed to the
static identity the artifact actually carries. Until one of those happens,
FR-042-AC-6 carries a trace tag whose test does not establish the clause that
commit added.

### Trace-tag binding

Every `#[trace(...)]` id in the two scoped files resolves in `spec/`:
`TC-121` at `spec/model-linking/tests.md:116` and `FR-042-AC-1..AC-9` in the
FR-042 acceptance table. No tag in the diff cites `FR-042-AC-10` or any absent
id. The engine's `unmatched_tags` list contains three entries, all inherited
`IT-004` tags in `tests/fixture_audit.rs`, none in this scope.

The review brief warned that Quire binds only one `TC-` id per trace attribute.
That is not the behaviour of the engine this repository runs. `TC-023`,
`TC-030`, `TC-034` and `TC-081` appear in this repository only in non-first
position inside multi-id attributes, and `quire coverage` reports all four
backed; `binding_census` reports 632 of 632 Rust tags bound. The warning is
recorded as checked and not reproduced. It is moot for this diff in any case,
because every new attribute here carries exactly one `TC-` id.

### FND-002 — matrix rows and their verified evidence state

`spec/model-linking/tests.md` marks all eleven rows `Planned`. Verified state:

| Row | Verified evidence | Honest status |
| --- | --- | --- |
| TC-121 | Backed; tagged tests across ten files, including both scoped files, all passing | Understated |
| FR-042-AC-1 | Backed; `native_payment_retry_emission.rs` and four other files | Understated |
| FR-042-AC-2 | Backed; `protocol_artifact.rs` only | Understated |
| FR-042-AC-3 | Backed; six files, none in this diff | Understated |
| FR-042-AC-4 | Backed; seven files including `native_payment_retry_emission.rs` | Understated |
| FR-042-AC-5 | Backed; eight files; the observed-repeat-guard clause added by `d62ee1b` is newly and directly covered | Understated |
| FR-042-AC-6 | Backed for the delivery/attempt/effect separation clauses; the per-occurrence ordinal clause is NOT covered (FND-001) | Partly understated, partly unbacked |
| FR-042-AC-7 | Backed; nine files | Understated |
| FR-042-AC-8 | Backed; six files including both scoped files | Understated |
| FR-042-AC-9 | Backed; five files including `native_payment_retry_emission.rs` zero/exact/one-short retry budgets | Understated |
| FR-042-AC-10 | No trace tag anywhere; requires the real quire-protocol IT-001 consumer handoff | Honestly Planned |

Nine rows plus TC-121 are conservative rather than false: a Planned row backed
by a passing test understates evidence, which is the safe direction, and the
matrix's own prose gives the reason (the producer-to-consumer handoff of AC-10
is open, so full FR-042 acceptance is not claimed). No row claims evidence it
does not have, and `quire coverage` reports zero `status_lies`. This analysis
records the repair as needed and does not make it; `spec/model-linking/tests.md`
is being repaired on a different branch.

### FND-003 and the observed-guard implementation

The capability is owned by requirement text. `d62ee1b` added the paragraph
admitting an observed Boolean repeat guard over the selected fragment, the
ownership/visibility/anchor obligations, the decision-instant scope rule, and
the partition-and-progress paragraph. The implementation matches those
sentences and does not exceed them:

- `families.rs` routes a non-constant guard to `decisions::partition` and keeps
  the constant path unchanged, so no previously refused constant shape changed
  meaning.
- `decisions.rs` generalises `partition` over a `Branches` enum and reuses the
  same `guard` extraction, locus charge and `Entries` accounting for both a
  choice case and a repeat's continuing branch.
- `received.rs` accepts `ControlKind::Repeat`, takes the owner from the
  repeat's `by` role, and keeps the `evaluation_anchor == Anchor::Control(decision)`
  test, which is exactly the "repeat's own anchor" sentence.
- The continuing body keeps the true-constant-guard progress obligation;
  `Progress::None` stays `Invalid::Control` and an unproved body stays
  `Unsupported::FamilyProof`.
- A zero maximum still takes the exhausted branch without a body obligation,
  matching "the authored finite maximum ... remain prerequisites of an observed
  guard, not substitutes for it".

No implemented behaviour in the diff lacks an owning sentence, and no new
public API, stub, `todo!` or placeholder appears; the three modified modules
are private to native emission.

FND-003 is the one place where the requirement claims more proof than the code
performs. The repeat's two branches are the authored guard and its exact
negation, pushed as `Op::Not(root)`, so `arena.partition` can never report a
hole or an overlap for a repeat. The code comment says so honestly. The
requirement sentence "The compiler SHALL establish that partition before
admitting the repeat" and the AC-5 phrase "an unproved one returns FamilyProof"
should be read as the visibility/ownership/atom-admission obligation they
actually discharge, or reworded.

### Neighbouring documents

No accepted document still forbids what FR-042-AC-5 now requires.
`docs/compiled-protocol-v1.md:394-401` and
`spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:122-148` were
amended in the same scope, and a sweep of `spec/` and `docs/` for surviving
statements that a repeat guard must be constant, literal, closed or static, or
that dynamic repeat guards remain unsupported, returns nothing. The wire
contract already modelled `repeat.guard` as a value handle
(`docs/compiled-protocol-v1.md:301`), so the observed guard needs no format
change. FR-042's composite-visible rule
(`Unsupported::FamilyProof` for `a and b`) is preserved for repeats and is
directly tested.

### Delivered scope against issue #39

This branch delivers the payment-retry leg of #39's second acceptance item and
nothing else. Split shipments and refunds are not delivered here: the fixture
names shipment-delivery receives `S1`/`S2` and refund events `R1`/`R2`, but
those are minimal event stubs supporting the retry scenario, not the split
shipment or refund choreography. #39's third acceptance item — causal and
visibility assumptions, captured obligations, commit/recovery semantics and
source correspondence for PT02 conformance and OB monitoring — is untouched.
FR-042-AC-10's real quire-protocol consumer handoff is untouched.
Remaining work: #39, #40.

## Coverage

`flock /tmp/quire-heavy-check.lock nice -n 10 quire coverage --scope . --json`
(quire 0.31.0, cli `4f6ed024`, engine `0.46.0@ca7362d4`) reports
`{"backed":374,"total":383}`, zero `status_lies`, six raw unbacked rows of
which two are `no_source_symbol` vocabulary exemptions, 20 untracked
`NFR-007-M-*` symbols and three unmatched `IT-004` tags. `TC-121` and
`FR-042-AC-1` through `FR-042-AC-9` are reported backed; `FR-042-AC-10` is
reported unbacked.

Test evidence: `flock /tmp/quire-heavy-check.lock nice -n 10 cargo test
--target-dir /tmp/quire-gap-target --jobs 1 --test native_payment_retry_emission
--test native_choice_emission -- --test-threads=1` gives 22 passed / 0 failed
and 9 passed / 0 failed.

No plan bundle covers protocol-artifact emission, so plan reconciliation is
inapplicable. The optional semantic review was not run; this analysis inspected
the amended sentences against the code and tests directly for the repeat-guard
and attempt-identity clauses only. No source, test, requirement, document or
matrix file was modified by this review.
