---
id: SR-520
title: "Base review of the M-6a owner-question rulings"
type: SpecReview
analysis: base
scope: "PR #353 diff against main: ADR-011, ADR-012, ADR-013, FR-062, FR-065, FR-079, FR-088, US-014, TC-203, TC-204, TC-259, spec.md, tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-259
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
---

## Summary

The diff records OQ-1 to OQ-7 and SG-1 to SG-6 in the sections it names, and
no reference to the removed ADR-011-OQ-1 remains outside `spec/reviews`. It
has no GitHub closing keywords and no compatibility or migration wording. The
`Until …` sentences describe blockers, not fallbacks.

Two rulings are only partly applied. The OQ-7 type-node rule is not carried
into FR-088-AC-7 and TC-259, which still require structurally identical type
declarations in two packages to get distinct ids. The SG-2
`capability_report` rule is not carried into ADR-012, which ADR-011's E3 row
cites as its authority, or into ADR-013's crossing table. Three references to
the old M-6b `lowering` deletion remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-088-AC-7 says structurally identical type declarations in two packages get distinct ids "because each declaration's owner is part of its preimage". OQ-7, ADR-013 O-04 and ADR-011 E3 say `scalar_type`, `composite_type` and `bounded_domain` nodes are keyed by content with no package scope and share one id across packages. A type declaration has no owner: the preimage `Owner` arms are source, definition and model owners only (QSpec `node-identity-preimage.schema.json` at 2449ceb). FR-322's `declaration` member is a package-local `{qualified_name}` (FR-322:160-172). TC-259 step 3 and its title, and the `spec/tests.md` TC-259 row title ("package-scoped"), repeat the old rule. TC-259 is marked passed (#300), so the test behind it asserts a rule the ruling reverses. Fix: rewrite AC-7 and TC-259 so that two declarations with equal structure and equal qualified names share an id across packages, declarations with different qualified names differ, and cross-package distinctness comes from `PackageNodeKey` (T-3). Drop "package-scoped" from both titles. | FR-088-AC-7; TC-259 title and step 3; spec/tests.md:145; ADR-013 O-04 (:178); ADR-011 E3 (:356) |
| FND-002 | high | SG-2 is not applied outside ADR-011. ADR-012 still says the checker records `Requirements` in `capability_report`: :597, the §8 Package row (:742, "requirements carried in `capability_report`"), OBS-012 (:818) and the §13.5 answer row (:999, "what ADR-011 S3 records in `capability_report`: exactly one entry per checked item"). ADR-011's E3 row cites that row as its authority. ADR-013's crossing table (:589) has capability values crossing QSL check → v2 package in `capability_report`. ADR-011's `check::capability` placement row (:849) says "`capability_report` is recorded at E3". Under SG-2, `capability_report` is FR-322's feature-level report, so these statements contradict it. Fix: in ADR-012 :597, :742, :999 and ADR-011 :849, say "per-item requirement records", and add to ADR-012 §13.5 that v2 `capability_report` is FR-322's `{feature, disposition}` report. In ADR-013 :589, remove capability values from the v2 package crossing and name their real carrier (see SR-521 FND-001). | ADR-012 :597, :742, :818, :999; ADR-013 :589; ADR-011 :849 |
| FND-003 | medium | Old M-6b `lowering` deletion text remains. FR-075:25-30 says deleting the catalog is #217's, "with SEAM-1 (ADR-011 §6.2, the `lowering` row; §7.3 M-6b)", and FR-075:186 says the same. The `spec/spec.md` FR-075 row (:455) says "deleting the lowering-target catalog is #217's". ADR-012 :803 says `--target` and `ProjectionTarget` "are deleted with SEAM-1 (ADR-011 M-6b)". Fix: change each to "deleted with `lowering` in ADR-011 §7.3 M-6a, once the skeleton spine is green". | FR-075:25-30, :186; spec/spec.md:455; ADR-012:803 |
| FND-004 | medium | ADR-013 :202, the member-type row, still says "`declaration` is a `NodeKey`, unique across packages (O-04)". That contradicts O-04 and T-3 as amended. Fix: "`declaration` is a `NodeKey`; a reference into another package is a `PackageNodeKey` (T-3)". | ADR-013:202; ADR-013 T-3 (:808) |
| FND-005 | low | FR-079's Description still opens with "when it replaces the fixed `ProjectionTarget` catalog with the registry". The new paragraph says that replacement never happens, so the requirement's trigger can never occur. Saying it "holds while `lowering` exists" is accurate but leaves an FR that no change can exercise. Fix: set FR-079's status to retired with `lowering` (M-6a), and keep TC-204's existing test as current-catalog evidence only. | FR-079 Description and Status; TC-203; TC-204 |
| FND-006 | low | Not edited here, because another in-flight PR owns FR-091's open questions: FR-091-OQ-2 and FR-091-OQ-3 are now stale. §2.4 says where a source unit's lock evidence comes from. OQ-3 cites the `name@version` preimage (QC-18) that this PR removes. | FR-091:401-414; ADR-011 §2.4; ADR-013 QC-18 |
| FND-007 | low | §2.4 records QSL's `DefinitionLock` revision drift (`1-draft.1` against QSpec's `1-draft.2`). That is a point-in-time claim about code (src/value/definition.rs:326, :537), and it will go stale without anyone editing the ADR. Fix: move the drift into the accessor ticket (SR-522 FND-001), and state in §2.4 only that the catalog is replaced by the accessor. | ADR-011 §2.4 (:505-508) |

## Resolution

Fixed in the PR: FND-001 (FR-088-AC-7, TC-259 and the tests.md row now state content keys with no package scope), FND-002 (ADR-012 lines on `capability_report`, ADR-013's crossing row and ADR-011's `check::capability` row now say per-item requirement records), FND-003 (FR-075, spec.md and ADR-012's backend-argument row now say M-6a), FND-004 (ADR-013 member-type row), FND-007 (the accessor replaces the catalog's restatement).
Left: FND-005 (FR-079 keeps its text while `lowering` exists; it retires with it). FND-006 belongs to the in-flight FR-091 rulings PR.
