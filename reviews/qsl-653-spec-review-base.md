---
id: SR-2444
title: "Spec review of quire-spec-language PR #663 (QSL-653): FR-100, FR-286-AC-6, ADR-013 and ADR-014 B-4 serialized authority"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2f0b659461cfc40a8d31e3efa09d32032e4d42e8; PR #663 spec diff against origin/main bfeb258c; spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, spec/tests.md TC-770; checked against quire-specification main fbd6ea0 FR-331, FR-271, FR-301"
review_set: subset
---
# Spec review of quire-spec-language PR #663

## Summary

Ticket: QSL-653.

- **FR-100:** states that `spine::run` takes the caller's `&Cancel`.
- **FR-286:** adds two statements and AC-6, which covers `from_run` and the driver-side document.
- **ADR-013 and ADR-014 B-4:** correct the serialized authority to QSpec FR-331's shape. Proof bounds are in `items[].extent.bounds` and `temporal.subject.proof_bounds`. Limits are in the item's temporal and probabilistic limits and in `deadline_ms`. There are no request-level `domains` or `limits`.

Examined:
- FR-100 statement paragraph, line 47 (examined)
- FR-286 statements on `from_run` and on driver failures, lines 118-128 (examined)
- FR-286-AC-6 (examined)
- ADR-013 serialized-authority row, line 805 (examined)
- ADR-014 B-4 row, line 132 (examined)
- QSpec FR-331 `request` member, line 58 (context_only)

## Verdict

**CONDITIONAL**.

The ADR-013 and ADR-014 edits match QSpec FR-331 line 58 exactly. No other QSL spec text still names a request-level `domains` or `limits`.

FR-286-AC-6 is testable and is tagged by TC-770.

The FR-286 statement that `unsupported_construct` "needs no new code" relies on a catalog reading that QSpec FR-271 contradicts (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-286 statement and AC-6 fix `unsupported_construct` as the code for a driver's unsupported engine. QSpec FR-271, the authority QSL reads its catalog from, separates `unsupported_construct` (a source form prohibited by the selected profile) from `unimplemented_capability` (valid meaning that the selected producer does not implement). An engine that is named and valid but not built is the second case. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:125-128 |
| FND-002 | low | The FR-100 edit is one unwrapped 200-character line inside a wrapped paragraph. It also says "a cancel at any stage or during the call", which no test backs for the call or for any stage after S1 (see the code-review SR-2442 FND-001). | spec/functional/FR-100-run-a-named-function-through-the-spine.md:47 |

## Dispositions

Round 1, reviewed at 0ae852bf192acfd086f7bf7c0d2db2596be2fecd.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 71423c552: FR-286 now names QSpec FR-271's `unimplemented_capability` for an unbuilt engine, never `unsupported_construct`, in category unsupported, exit 21. FR-286-AC-6 and TC-770 step 6 match. |
| FND-002 | fixed | 71423c552: the FR-100 paragraph is rewrapped. Its "S1 to S4 or during the S6a call" claim is now FR-100-AC-12, backed by TC-452 step 5. |
