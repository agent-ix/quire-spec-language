---
id: SR-15730
title: "spec review of STD-290 evidence kind of a probabilistic request item (QSL PR #1249)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@621148a34510019fc06093d15233d9d2d1afff98; spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md, spec/test-cases/TC-1163-a-probabilistic-claim-s-item-names-its-evidence-kind.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1163
    type: reviews
---
# SR-15730: spec review of STD-290 evidence kind of a probabilistic request item (QSL PR #1249)

## Summary

Ticket: STD-290. The diff is `origin/main...621148a34` and is spec only. It touches FR-075, FR-195 and the new TC-1163. The merge base is 48108c9c1; main has since moved to 43f4e2086 (STD-301, #1236), so main is not an ancestor of the head. A trial merge is clean. The reviewer checked the diff against QSL main (FR-075, FR-186, FR-062, FR-395, ADR-012 §13.5, ADR-024 SV-9) and QSpec main 5ee0071 (FR-290 "Evidence kinds", FR-407 "Obligation identity" and the exact-only forms, FR-408).

These claims hold:

- The kinds are spelled `exact` and `statistical`, as QSpec FR-290 and the `qsl-route` `EvidenceKind` spell them.
- The FR-195 default (`exact` when `exact_only`, else `statistical`) agrees with QSpec FR-407: a claim with no confidence parameters "is checked with exact evidence only", and the reachability/until and expected-reward rows are the exact-only forms.
- Recording the kind on the requirement record is grounded in ADR-012 §13.5, which says each record holds, for `probabilistic-satisfaction`, its exact evidence kind. Keeping it out of the normal form and the obligation identity agrees with QSpec FR-407 ("The evidence kind ... are request settings and are outside the obligation identity").
- The both-kinds rule implements QSpec FR-290's "One claim may be requested twice, once per evidence kind; the two items have separate terminal records". QSpec fixes no order, so `exact` before `statistical` (bytewise) is a free choice and is consistent.
- Candidate derivation for each item follows FR-075's existing exact-match rule, so a `statistical` selection on `FairTerminates` reaches EN-4's gate and settles `unsupported`, as FR-195 and QSpec FR-408 require.
- TC-1163, FR-075-AC-18 and FR-195-AC-6 appear on no other open PR head (46 open PRs scanned) and not on main. No other open PR touches FR-075, FR-195, FR-062, FR-186 or FR-395. There is no status wording and no compatibility layer.
- `QSPEC_DIR=~/dev/quire-specification make check-spec-validation` at the head exits 0 with no diagnostics on the touched files.

Lead defaults:

- Default 1 (claim default plus request override) is sound for the reasons above.
- Default 2 (bytewise order) is sound.
- Default 3 (request-wide selection) is acceptable.
- Default 4 (FR-062 not edited) is right about FR-062: clause records are stated in their family FRs (FR-104, FR-326), not in FR-062's value-validity section. But no FR states the probabilistic requirement record at all (FND-002).

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: Two items per record breaks the request-index contract. FR-075's builder statement still says it "SHALL return one `RequirementItem` (design name) per record" with the request index "its position in that order", and the same paragraph lists no evidence-selection input. FR-075 also says "An item's request index is its position in the package order". FR-395 defines the request index as the record's position in the package order, and ADR-012 §13.5 says `request_index` is the bytewise order of the record keys. Vector: a package with `P95`'s record at key k1 and a `value-validity` record at k2 > k1. With no selection the indices are 0 and 1. With both kinds they are 0 (`exact`), 1 (`statistical`) and 2 (`value-validity`), but FR-395 still gives the value-validity member request index 1, which now names `P95`'s `statistical` item, so FR-075-AC-17 and FR-395's pairing fail. The new rule "the evidence selection SHALL leave those items unchanged" is also false for that item's index. <br>Fix, planner decision: (a) keep each record's index and append the second-kind items after the last record (indices N, N+1, ...); (b) key items by (record index, evidence kind) and amend FR-395, ADR-012 §13.5 and FR-331's terminal-record key; or (c) redefine the request index over items and amend FR-395 and ADR-012 §13.5 to match. In every case, update the builder statement's input list and its "one per record" sentence, and make AC-18 pin the value-validity item's request index under every selection. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:123-128, spec/functional/FR-075-compute-candidates-from-registered-backends.md:150-171, spec/functional/FR-075-compute-candidates-from-registered-backends.md:179-181 |
| FND-002 | medium | minor: No FR states that a checked probabilistic claim has a requirement record. FR-195's new bullet says S3 records the evidence kind "on each checked probabilistic claim's `probabilistic-satisfaction` requirement record", but FR-062 states only value-validity records, and FR-186 says only that the claim is of kind `probabilistic-satisfaction`. So the record's key, extent, result bound, conversion context and path condition are unstated, and FR-075's builder needs every one of these to write the item AC-18 tests. FR-062 does not need to change. The house pattern is the family FR stating its record, as FR-104 and FR-326 do for clauses. <br>Fix: add one record rule to FR-186 (or FR-195): one record per checked claim, keyed by the claim's occurrence key, with its extent, no result bound or conversion context, an empty path condition, and the evidence kind. If the planner wants this as a follow-up instead, file a ticket and state in the PR that AC-18 depends on it. | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:119-124 |
| FND-003 | low | minor: FR-195-AC-6's last clause cannot fail. "`P95` and the same claim with its record's evidence kind replaced have equal normal forms" compares a `CheckedProbabilisticClaim` member that is computed before the record exists, so changing a record field cannot change it. The bullet's other claim, that the kind is not in the obligation identity, has no test. <br>Fix: assert that `CheckedProbabilisticClaim` has no evidence-kind member, and that `P95`'s obligation identity is equal across the no-selection, `exact` and both-kinds requests of TC-1163 step 3 (QSpec FR-407-AC-6). | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:150 |
| FND-004 | low | minor: AC-18 wording. "With both kinds it writes four items", but the package also holds a value-validity record, so it writes five. "The later claim's items after the earlier's" does not say what "later" means; occurrence-key order is meant. Under the `exact` selection, the AC does not state the candidates, which is the concrete no-substitution check: `P95`'s `exact` item has the EN-5 registration only. TC-1163's expected results repeat the same gaps. <br>Fix: say "four probabilistic items", "in the claims' occurrence-key order", and give the candidates of each item under each selection. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:346 |

## Verdict

Not mergeable as is. FND-001 is a contradiction with FR-395 and ADR-012 §13.5 that a test of FR-075-AC-17 would hit as soon as a package mixes probabilistic and value-validity records. Its fix needs a planner decision. FND-002 needs a planner decision on whether to add the record rule here or in a follow-up. FND-003 and FND-004 are small wording fixes. Main must also be merged into the branch (one commit behind; the merge is clean).

The FR-195 default, the kind spellings, the both-kinds ordering and the candidate derivation are consistent with QSpec.
