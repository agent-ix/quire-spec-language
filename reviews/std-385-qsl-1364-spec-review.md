---
id: SR-16256
title: "Spec-review of the hybrid reader and encoding fix-forward (quire-spec-language PR 1364)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@419f1ce75ec31e09561b0fd5f7d4133e5c71977b (origin/main...HEAD); spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md; spec/test-cases/TC-1187-a-hybrid-declaration-checks-to-a-classified-model-and-routes-its-claim.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1187
    type: references
---

# Spec-review of the hybrid reader and encoding fix-forward (quire-spec-language PR 1364)

## Summary

Ticket: STD-385. This PR fixes forward SR-16261, the post-merge review of
#1350. The companion QSpec PR is quire-specification #414 (SR-16255).
Reviewed `git diff origin/main...HEAD` at head 419f1ce7 (3 files,
+114/-10, of which 70 lines are the committed SR-16261 file). Main is an
ancestor of the head.

Gates run by the reviewer with `QSPEC_DIR` at QSpec #414 head 59cad4f4:

- `make check-spec-validation`: exit 0 (warnings only, all in files this
  PR does not touch).
- `cargo test -p qsl-package --lib checked_v2`: 41 passed, 6 failed. Every
  failure is `UnsupportedNodeTag` at `/semantic_graph/nodes/39/node_tag`,
  the first `hybrid` node of QSpec's `positive-all-families.json`. That is
  the reader code this PR specifies, and nothing else causes it.

Prior findings:

- FND-001 is fixed. FR-447 now has I2-reader rules (read back into
  `CheckedHybridModel`/`CheckedHybridClaim`, refuse in FR-481's order with
  FR-481's code and locus, layout `malformed_wire`) and AC-6, which owns
  FR-481's reader refusals in QSL.
- FND-002 is fixed. `Holds(Boolean(b))` is `quire.op.temporal.true`/`false`
  and every other `Holds(t)` is `quire.op.temporal.holds`, pinned by AC-7.
  This matches FR-481 "Hybrid claims".
- FND-003 is fixed by refusing every profile but infinite-trace at S3,
  which is a scope narrowing (FND-001 below).

The `-(2 * a)` spelling now matches QSpec in AC-5 and TC-1187.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [scope-deferral] S3 now permanently refuses every hybrid claim whose `using` profile is not `quire.temporal.infinite-trace/v1` (`unsupported_construct`/`declaration-form`), and AC-7 pins the refusal for `timed/v1`. This removes hybrid claims under the dense-time profile and the bounded profiles. Before this PR, S3 admitted them. The cause is that S4 emitted only integer intervals, not that the claims are meaningless. `HybridInterval` holds only `u64` seconds, so even the admitted profile cannot express a fractional bound such as `always [0, 1/2]`. The PR's own lead default does not excuse a narrowing under the standing rule. Fix: admit `timed/v1` hybrid claims, give `HybridInterval` exact rational bounds and emit them in FR-370's timed form, and replace AC-7's refusal with an admitted `timed/v1` vector. Settle the bounded profiles only on Peter's explicit answer. Land this together with SR-16255 FND-002. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:252-256; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:157; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:385 | wrong-requirement |
| FND-002 | medium | [soundness] AC-6 requires QSL's I2 reader to admit QSpec's `positive-all-families.json`. At #414's head, that fixture's `Tank` invariant and guard are the text literal `"audit"`, which FR-481 checks 3.2 and 3.3 refuse `ill_typed`/`non-boolean-root` (SR-16255 FND-001). As specified, AC-6 cannot pass for a reader that follows FR-481's order. Fix: no QSL text change is needed, but AC-6 is not satisfiable until SR-16255 FND-001 lands; record that dependency in the QSL code ticket. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:384 | correct-requirement-no-evidence |

## Verdict

Not mergeable. There is one high finding: the profile refusal is a scope
narrowing (FND-001). There is one medium finding: AC-6 depends on a QSpec
fixture defect (FND-002). The read-back rules and the true/false encoding
are sound, and spec validation passes. The conformance failures are only
the missing reader code.
