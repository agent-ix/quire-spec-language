---
id: SR-16261
title: "Post-merge spec-review of S4 hybrid model emission (quire-spec-language PR 1350)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@425bab9bb33234f50fcce7dd9bec58e8a89620ee (vs parent); spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md; spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md; spec/test-cases/TC-1187-a-hybrid-declaration-checks-to-a-classified-model-and-routes-its-claim.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1187
    type: references
---

# Post-merge spec-review of S4 hybrid model emission (quire-spec-language PR 1350)

## Summary

Ticket: STD-385 (spec gap behind QSL-900), QSL half. The QSpec half is
quire-specification #400 (FR-481), reviewed in SR-16260. The PR was merged by
its author without a review. This is a post-merge review, so findings are
fixed forward. Reviewed `git show 425bab9b` against its parent (3 files,
+90/-42), against QSpec FR-481, FR-370, FR-331 and FR-193 at QSpec main
059998c4, and against QSL FR-411 (the precedent for an emitted-node FR).

Gates run by the reviewer on QSL main 425bab9b with `QSPEC_DIR` at QSpec
059998c4:

- `make check-spec-validation` exits 0.
- `cargo test -p qsl-package --lib checked_v2` gives 46 passed.

Examined and clean:

- The S4 refusal and the in-process-only rule are fully removed, and nothing
  in FR-447, FR-250 or TC-1187 still refuses or narrows the process route.
- The lowering of `CheckedHybridModel`, `CheckedMode`, `CheckedTransition`,
  `ContinuousVariable` and `HybridTerm` matches FR-481 node for node. The
  `class` is carried as recorded at S3. Indices name targets and reset
  variables.
- The request item names the emitted `temporal_clause` node and its `claim`
  occurrence, consistent with QSpec FR-331 "Items".
- FR-250 already obliges QSL to check every trace against the
  `CheckedHybridModel`, whichever provider returned it. This is correct, and
  it is the behaviour QSpec FR-193 should state (SR-16260 FND-005).
- `package_id` covers every hybrid node. AC-4 and AC-5 and TC-1187 steps 4
  and 5 follow the new behaviour. No ids were added.

QSpec defects that QSL output runs into are recorded only in SR-16260. The
main ones: a bare-variable rate such as `x' = v` is emitted as a reference
to a parameter node, which FR-481's reader refuses (SR-16260 FND-002); and
the `-2 * a` versus `-(2 * a)` example mismatch (SR-16260 FND-010).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [scope-deferral] No QSL requirement makes QSL's I2 reader read the emitted hybrid nodes. FR-411's precedent says "QSL's I2 reader SHALL read that node back" and refuses breaks of the QSpec rules "in FR-476's reader order", with an AC. FR-447 adds emission only. AC-5 hands admission to "QSpec's v2 reader", and no AC covers reading the nodes back into a `CheckedHybridModel`/`CheckedHybridClaim` or FR-481's hybrid-step refusals (FR-481-AC-4 and AC-6 have no QSL owner). QSpec #400's decision 7 defers the all-families fixture "until QSL's reader admits `hybrid`", with no QSL requirement or ticket that makes it do so. Fix: add the I2-reader rules and ACs (read back, refuse in FR-481's order) to FR-447 or to a new FR, so that SR-16260 FND-001 can land. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:364; spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md:84-88 | correct-requirement-no-evidence |
| FND-002 | medium | [ambiguous] It is not defined how `Holds(Boolean(b))` becomes a formula node. FR-447 lowers `true` and `false` in a claim body to `Holds(Boolean(..))` and emits each `HybridFormula` "as QSpec FR-370 formula nodes". FR-370 has its own `quire.op.temporal.true`/`false` operations, and FR-481 check 7 admits them, so one producer can emit `quire.op.temporal.true` and another `holds(literal true)`. The two packages have different `package_id`s for one claim. Fix: name the one encoding (for example: `Holds(Boolean(b))` is `quire.op.temporal.true`/`false`, and every other `Holds(t)` is `quire.op.temporal.holds`) and add an AC. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:263; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:349 | wrong-requirement |
| FND-003 | medium | [soundness] S3 does not restrict a hybrid claim's temporal profile. It admits any `using` profile, subject only to FR-123's interval rule, while S4 always emits integer-form intervals. Under `quire.temporal.timed/v1`, QSpec FR-370 refuses integer intervals `operation-member-mismatch` in the temporal step, so a claim S3 admits produces a package the reader refuses. Fix: state the profiles a hybrid claim may select and refuse the others at S3 with a span, or emit the timed form under `timed/v1`. Settle this together with SR-16260 FND-008. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:252-272; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:349 | wrong-requirement |

## Verdict

Not done as merged. Fix forward. There is one high finding (FND-001): the
reader half of the hybrid node form has no QSL owner, which is also what
QSpec's deferral waits on. There are two medium findings: an
identity-affecting encoding ambiguity (FND-002) and a profile hole
(FND-003). The emission rules, the request locus, the routing and the
FR-250 note are sound, and the spec-validation and conformance gates pass.
