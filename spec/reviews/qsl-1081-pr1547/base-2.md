---
id: SR-17220
title: "spec-review disposition of SR-17219 findings on spec capture emission (quire-spec-language PR 1547, head <sha removed>)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@<sha removed>; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md; spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md; spec/test-cases/TC-1267-a-temporal-clause-with-a-capture-checks-emits-and-is-observed-at-activation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: references
---

# Spec review disposition of SR-17219 (quire-spec-language PR 1547)

## Summary

Ticket: QSL-1081. Re-review at head <sha removed> of the fixes for SR-17219 FND-001 to FND-007. Each is resolved: FND-001 (`holds(n < 3 and c.value = n + 1)` bounds the sum in the formula, FR-104-AC-12), FND-002 (the requirement is the emitted clause node, so its key is the clause key and differs with the captures, consistent in FR-337, FR-218-AC-16, FR-383 "requirements holds the temporal clause nodes", FR-383-AC-17 and FR-322 content keying), FND-003 (FR-337-AC-13 replay, certificate and component checker, FR-337-AC-14 footprint; consistent with FR-338-AC-10 undefined-condition BadState), FND-004 (FR-337-AC-12 infinite-trace, bounded monitor `on origin`, second capture and unbounded-root precedence vectors; `CaptureRead` named in FR-126 outputs), FND-005 (slot counts 2/3 under `on each`, 1/2 under `on origin`, holds 4/3, matching the S4 levels), FND-006 (QSL-1092 names the CaptureRead engine work), FND-007 (initializer parenthesised). TC-1266 is owned by PR 1548 (hyper clause); this PR has no TC-1266 and TC-1267 is free on main and on PR 1548. `quire validate` on every changed file reports no errors. One new low finding remains.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-362 line 74 still says a checked clause "with a capture or under the timed profile" is refused `NotEmitted` under FR-337 "Clauses S4 does not emit". That section now says a capture is not a reason to refuse, so FR-362 contradicts it. Fix: drop "one with a capture or" from FR-362 (keep the timed profile and protocol fairness). Also cosmetic: FR-337-AC-14 has a duplicated trailing `Test (TC-1267)` cell and FR-337-AC-12 has no verification cell; add/remove so each row has one. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:74 | missing-requirement |

## Verdict

PASS for SR-17219: all seven findings resolved with no new contradiction in FR-218-AC-16, FR-383/FR-322 keys, FR-337-AC-13 versus FR-338/FR-339, or FR-126. FND-001 (low) is a one-line stale cross-reference to fix in the same PR; it does not reopen any SR-17219 finding.
