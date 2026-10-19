---
id: SR-12162
title: "QSL-529 gap analysis of PR #997 (FR-338 AC-1 to AC-7, TC-525, ADR-018 section 12, ADR-019)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3cb530c9b1cfbd6940888c0c038e445c293cbec2; PR #997 diff against origin/main; spec/functional/FR-338-check-an-en-1-closure-certificate.md; spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md (section 12, PC-3, PC-5, LA-2, LA-3); spec/decisions/ADR-019-strong-fairness.md (context only); qsl-analyze/tests/it/closure_certificate.rs; qsl-eval/tests/it/automaton_key.rs; qsl-replay/src/certificate/closure.rs (unit test)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-525
    type: reviews
---
# QSL-529 gap analysis of PR #997

## Summary

Ticket: QSL-529. PR: quire-spec-language#997, head 3cb530c9. Planless gap
analysis: plan completion was not assessed. The AC-to-test trace comes from the
diff's `#[trace]` tags. All 14 new tags read `TC-525`, and none names an
FR-338 AC, so the computed matrix shows no FR-338 AC covered. TC-525 asks for
`#[trace("TC-525", "FR-338-AC-n")]`. Leaving out the AC ids is honest here,
because no AC is verified exactly as written:

| AC | State at 3cb530c9 |
| --- | --- |
| FR-338-AC-1 | Exercised on a substitute: the two-counter Counter subject with `always holds(c.value <= 3)` instead of ADR-018 section 6's `ConfigVersion`. `a_proved_invariant_carries_a_certificate_the_core_accepts` compares the certificate with an independent BFS enumeration and checks acceptance. Declared gap. |
| FR-338-AC-2 | The deadlock half runs exactly as written: the Counter subject with no `terminal` rejects `BadState` at value 3, and `c.value <= 3` is accepted. The SuccessorMissing and InitialMissing halves use the Counter substitute for AC-1's certificate. Each locus is computed from the Counter arithmetic, not from the checker, so the oracle is genuine. AC-2 as one row is only partly met. |
| FR-338-AC-3 | Exercised on a substitute: Counter with `pre CanInc { 6 mod (2 - self.value) >= 0 }` instead of `test/tallies`. Declared gap. |
| FR-338-AC-4 | Only the safety form: the deadlock full-key oracle byte for byte, TP-1 bodies `["safety",[]]` and the carried root, clause-root ordinals `"0"` and u64::MAX, and order independence. The bounded, window and offset vectors are not covered, because no EN-1 phase decides those forms. |
| FR-338-AC-5 | Only the safety-form negatives, in the translation unit: foreign context, profile and activation, non-canonical, discriminator, unknown or wrong tag, missing or extra members, unsorted or duplicate sets, `"01"`, NodeId used as a path, unresolved occurrence, interval debt, past and acceptance members, unreachable branch. At checker level only foreign context and foreign digest domain are exercised. |
| FR-338-AC-6 | Not covered (declared). |
| FR-338-AC-7 | Not covered (declared). |

Declared gaps are not raised as findings: AC-6 and AC-7, the AC-1 and AC-3
substitutes, the FR-337 fixture identity, and FR-338's missing outcome for a
refused key. ADR-019 strong fairness does not apply to the TP-1 and deadlock
items this PR decides. It matters only to FR-338's non-progress check, which
these items skip by rule.

A Linear comment on QSL-529 asks for extra scope: past-over-future operators,
and #957 key octets. It is untrusted, it is the planner's to decide, and it is
not used here.

Code with no owning requirement: none. Every new public item traces to FR-338,
FR-126 or FR-127. No stub or tautological test was found. Each oracle is either
an independent enumeration or a literal octet string.

## Verdict

Coverage matches what the PR declares, with two adverse cases missing that bear
on soundness. Both are paired with code-review findings in SR-12161. Neither is
a major in its own right. The blocking defect is SR-12161 FND-001, and its fix
must bring the FND-001 test below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No adverse test for a subject with no initial state at the checker. FR-338 accepts only a certificate that contains every initial product state, and FR-127 settles no initial state as `Inconclusive(NoInitialState)`. No test offers `check_closure` or `settle_model_check` a `Holds` over an empty `initial`. Today that path returns `Accepted` and `Certified` (SR-12161 FND-001). | qsl-analyze/tests/it/closure_certificate.rs |
| FND-002 | medium | The settlement test pins an unbound package. `a_proof_settles_certified_only_when_its_certificate_is_accepted` passes `DigestRecord::mint(PackageSemanticV2, [0; 32])`, which is not the world's package, and still expects `Certified`. No test checks that a request over another package fails to certify. FR-338 Inputs bind the request to the recompiled package (SR-12161 FND-002). | qsl-analyze/tests/it/closure_certificate.rs:443-460 |
| FND-003 | low | AC-5's "refuses before the key is used for membership" is shown at checker level only for a foreign context and a foreign digest domain. No `check_closure` test offers a malformed, non-canonical or unsorted key and asserts `ShapeMismatch` at that state. The translation-unit tests cover `admit`, but not the checker's use of it. | qsl-analyze/tests/it/closure_certificate.rs:389-429 |

## Dispositions

Round 1, reviewed at 2071dfd1323b0a953b0cdeda5d663a313bc58276. The fix
commits are e55788157 and 2071dfd13. The round adds no new gap finding.

STD-185 adds FR-338-AC-8. Its refused-key half is exercised by
`a_refused_key_rejects_the_certificate_at_the_least_refused_state`, which is
tagged `TC-1080` only. That tag is honest, because the subject is the Counter
substitute and the unresolved-root half is not done. The PR body states that
gap (FR-337, spec gap 1). No test claims FR-337-AC-5, TC-1079 or the
unresolved-root refusal.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e55788157 |
| FND-002 | fixed | e55788157 |
| FND-003 | fixed | e55788157 |
