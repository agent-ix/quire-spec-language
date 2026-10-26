---
id: SR-60028
title: "spec-review of FR-281-AC-4 uncertified proof via EN-1 under partial-order reduction (quire-spec-language PR 1601)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5ba21fb7b191e48433cc80e663baad651a958c08; spec/functional/FR-281-analyze-claims-with-in-process-engines.md; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md; spec/test-cases/TC-763-analyze-settles-rejected-certificates-unreproduced-counterexamples.md; spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md; qsl-analyze/tests/it/analyze.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---

# Spec review of QSL-1114 FR-281-AC-4 amendment (quire-spec-language PR 1601)

## Summary

One pass over `git diff origin/main...5ba21fb7`. The amendment is faithful. ADR-018 PC-1 and RU-6 and ADR-021 RV-1 name a partial-order-reduced proof as `uncertified` because the core has no checker for a reduced graph (not a checker that rejects; a rejection is `inconclusive`, PC-2). `settle.rs` maps `HoldsReduced` to `Proved{Reduced, Uncertified}` and `Holds` to `Certified`. The old wording (a test engine with no checker) cannot be built from tests/it, and EN-4 never settles `proved`, so EN-1 under reduction is the only layer-A path to the FR-281 Description case. Mutation: `HoldsReduced` to `Certified` fails the new test; the unreduced `Holds` path to `Uncertified` fails the new test and two others; both reverted. Focused `analyze::` tests (10), clippy -D warnings on qsl-analyze and string-edge are clean; quire validate shows no new warnings.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | TC-774 Expected Results step 1 still says 'the test engine's record is `proved` with certification `uncertified`'; the Procedure step 1 and FR-290-AC-1 were amended but this line was missed. Fix: say 'the EN-1 reduced-proof record'. | spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md:41 | wrong-requirement |
| FND-002 | low | The doc comment on `tc_774_plugin_proof_settles_trusted_and_kani_proof_certified` still says FR-281-AC-4's record comes 'from a test engine with no checker (no such engine exists, and analyze accepts no engine-supplied proof)'. That is now false for AC-4. The FR-290-AC-1 tag is correctly still withheld (the FR-281-AC-1 EN-5 record is blocked on QSL-574/575), so only the reason text needs updating. | qsl-replay/src/proof_result/plugin_tests.rs:1154-1160 | wrong-requirement |
| FND-003 | low | FR-281-AC-4's first half says 'the test engine's certificate altered in one member'. The tagged test a_rejected_certificate_is_inconclusive_and_an_accepted_one_certified instead hands EN-1 a certificate request for a different item (ShapeMismatch), which is a rejected certificate but not a one-member alteration. Pre-existing and not changed by this PR; it asserts the CertificateRejected cause, so the tag holds, but the AC wording is stronger than the test. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:273; qsl-analyze/tests/it/analyze.rs:209 | correct-requirement-no-evidence |

## Verdict

PASS. FND-001 to FND-003 (low) should be fixed in the same round or noted; none blocks merge.
