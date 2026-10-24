---
id: SR-12812
title: "Gap analysis of FR-165 state-graph claims (QSL-535, PR #968)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f414b7f16304cb757661adc907c524a534418d24; PR #968; spec/functional/FR-165, qsl-forms/tests/it/state_graph_forms.rs, qsl-semantics/tests/it/state_graph_claims.rs"
review_set: subset
evaluated_revision: "f414b7f16304cb757661adc907c524a534418d24"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
---

## Summary

Ticket: QSL-535. Planless. In scope: FR-165-AC-2 and the form part of
FR-165-AC-1. `quoin matrix --json` (quoin 0.28.3): FR-165-AC-1 tagged,
FR-165-AC-2 tagged, FR-165-AC-3 untagged (out of scope, not raised).
Plan completion: not assessed.

## Verdict

AC-2 is fully and strongly covered: each of the three named refusals asserts
code, cause tag, exact source text and cause arm, with sole-refusal checks;
`self`/`result` and a non-Boolean predicate are extra. The S2 tests check
span text of each predicate. AC-1's S3 form tests are weak oracles (FND-001).
The one-line FR-165 spec clarification is behaviour, not build log, and
agrees with the code except for non-leading temporal operators (SR-12811
FND-002). No stubs or untraced production code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | S3 AC-1 tests assert only `body().is_some()` for `from`/`target`/`to`: swapping `from` and `target`, swapping `unique path`'s `from` and `to`, or checking one predicate twice all pass. The tests use the ConfigVersion fixture only, so AC-1's `CanStillWin from (x.phase != Lost)` enum predicate and `InOneWay` over `M::Job` are never checked at S3. | qsl-semantics/tests/it/state_graph_claims.rs:104-135 |
| FND-002 | low | FR-165-AC-1 shows as tagged in the matrix although its two-instance and node-identity parts are untested (deferred to STD-162/QSL-863), and the requirement-record test is tautological: it builds the record from a caller-supplied node and checks that node back. | qsl-semantics/tests/it/state_graph_claims.rs:150-170 |

## Coverage

Examined FR-165-AC-1 (form part) and FR-165-AC-2 against the eleven S3 and
five S2 tests tagged TC-590. Ran them green. Reverse check: every new
production symbol in qsl-forms/src/state_graph.rs and
qsl-semantics/src/check/state_graph_claim.rs is reached by a tagged test.

## Dispositions

Round 1, at 5456c3b03f01941ac09a6aeca5bc3f7344fa4801.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Oracle part fixed: the S3 tests compare each slot's checked literals against a separately checked reference and assert the slots differ, so a from/target or from/to swap now fails. The §7 `CanStillWin` (G::Game) and `InOneWay` (M::Job) S3 fixtures are a legitimate deferral because no Game/Job domain package exists and AC-2's named refusals use ConfigVersion only. No follow-up ticket names them yet; QSL-863 (which needs the §7.2 game for AC-3) should take them. |
| FND-002 | still-open | The `#[trace]` tags dropped FR-165-AC-1, but the `///` doc comments on nine tests still say `FR-165-AC-1:`. `quoin matrix --json` at 5456c3b still binds FR-165-AC-1 to those tests (static_status `tagged`). Rewording the doc comments (for example "Part of FR-165 AC-1") would make the matrix honest. The tautological record check now also asserts a different node does not match. |

Round 2, at f797d36addb7be8a8ed3044eeb09debfbce1be3b.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | The test doc comments now say "TC-590, the form part of FR-165". At f797d36, `quoin matrix --json` reports FR-165-AC-1 `untagged`, FR-165-AC-2 `tagged` and FR-165-AC-3 `untagged`, so the matrix no longer overstates AC-1. |

Round 3, at b82b29bc8bee5d29ca40a5df3bb6e80eb2d4760b, against the merged FR-165 (AC-1 to AC-5), FR-125 Instancing and FR-391. `quoin matrix --json` reports AC-1 untagged, AC-2 tagged, AC-3 untagged (QSL-863), AC-4 tagged and AC-5 tagged; the tags are honest. AC-4's S1 and S3 halves each assert the form arm, `over` `None` versus `Some("c")`, and the refusal code, cause and source text at `eventually`. AC-5 uses the probeInt fixture's real operation parameter `amount`, so a missing-name refusal there means the parameter is not in scope, not that the name was mistyped; it then checks that `pre(...)` still refuses `expression-form`. No disposition change: FND-001's latest row stays `deferred` (the §7 Game/Job fixtures) and FND-002's stays `fixed`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-863: the lead recorded the §7 `CanStillWin` (G::Game) and `InOneWay` (M::Job) S3 fixtures on QSL-863. The oracle part was fixed in round 1. |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | `fairness_and_temporal_operators_are_confined_to_their_positions` asserts only that some diagnostic exists. It does not check its location (`;`, `fair` or `eventually`), which QSpec SG-R13/R15 and STD-176's AC-6 name. The code reports the right tokens (probed), but an S1 that reported at `fair` instead of the `;` would still pass. SG-R08 and SG-R18 do pin their offsets. | qsl-forms/tests/it/state_graph_forms.rs:325-335 |

Round 4, at 8fe62d1e35b3456c07f81bed858c67ea8e67c311. The new S1/S2/S3 vectors (SG-R08 to R11, R13 to R18) are tagged TC-590 and FR-165-AC-4. Once STD-176 lands, its S1 syntax-error vectors belong to the new FR-165-AC-6. SG-R01 to R07 and the SG-S vectors are not ported; most already have equivalents under AC-1, AC-2 and AC-4. No change to earlier rows.

Round 5, at f43e510325272eae2c120e9cc663324a2d0eecb9, against merged FR-165 AC-1 to AC-6 (STD-176, #986) and QSpec `graph-predicate`. `quoin matrix` reports AC-1 untagged, AC-2 tagged, AC-3 untagged (QSL-863), and AC-4, AC-5 and AC-6 tagged. The AC-6 tests pin every syntax-error location the AC names, including the merged spec's added `from (fair ...; P)` at `fair`. AC-4 now covers the interval, `let`, group-chain, `from`, `to` and leading-fairness placements at S3, each asserting code, cause and refusal text.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | `refused_at` asserts the `invalid_syntax` code and the exact byte offset of the named token for every AC-6 vector. |

Round 6, at 75fd07be96bb0818d8389c5af43c262c3f47d9a7. There is no change to the gap findings. The two new FR-165-AC-5 vectors (`let`-scoped names-first) discriminate: they fail on the round-5 code. The if/else vector asserts only the named text `n`, not its offset, but the first vector would catch an implementation that drops `let` scopes altogether.
