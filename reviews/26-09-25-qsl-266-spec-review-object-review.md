---
id: SR-639
title: "QSL-266 object review of requirement records, ClaimSite and the request builder"
type: SpecReview
scope: "agent-ix/quire-spec-language@64ee12700cd66bb17767a8e9090cbca114308364; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/functional/FR-062-implement-checked-family-contract.md; spec/functional/FR-075-compute-candidates-from-registered-backends.md; spec/test-cases/TC-160-checked-family-contract-shape.md; spec/test-cases/TC-449-request-builder-writes-one-item-per-requirement-record.md; qsl-semantics/src/family/contract.rs; qsl-semantics/src/check/mod.rs; qsl-route/src/request.rs; qsl-route/src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
---

## Summary

Ticket: QSL-266. This review audits the objects the diff introduces or
reshapes:
- the per-claim-site requirement record (FR-062);
- the design name `ClaimSite` in ADR-012's contract sketch;
- the `route` request builder `qsl_route::request::items_from_requirements`
  and its item (FR-075).

Each was checked against the code it names:
- `FamilyContract::requirements` (qsl-semantics/src/family/contract.rs:310);
- `key_requirements` (qsl-semantics/src/check/mod.rs:1123);
- `RequestWriter::item` and `RequestItem` (qsl-route/src/request.rs:98, 194);
- `Registry::candidates` and `CandidateOutcome::UnknownBackend`
  (qsl-route/src/lib.rs:474-480, 631).

What is right:
- The record's key and its `Requirements` members are defined.
- `RequestWriter::item`, `Registry::candidates` and the unknown-backend
  marker exist with the shapes FR-075 cites.
- FR-075's new `depends_on` edges to FR-062 and FR-097, and TC-160's new
  `verifies` FR-057, are correct.

What is wrong:
- `ClaimSite` is used once and defined nowhere.
- The builder's item is not a named type, and no member carries what
  generation needs (see SR-638 FND-001).
- FR-062's frontmatter lacks edges to specs it now depends on normatively.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `ClaimSite` appears once in the spec and code: in ADR-012's trait sketch, `fn requirements(checked: &Self::Checked) -> Vec<(ClaimSite, Requirements)>`. FR-062 part 4 says "paired with the checked site the claim covers", and FR-062-AC-4's oracle ("at its checked site", "at the `+` application") depends on it. Nothing states what a `ClaimSite` holds (a checked-tree `Location`/region, a node key, or a path), or how S3 turns it into exactly one occurrence key. Two occurrences of one node must give two distinguishable sites, or two different extents at one node cannot be keyed correctly (SR-640 FND-002). Define it in FR-062 as a design-level object: its members, its site-to-occurrence-key rule, and that it separates occurrences. Leave the Rust spelling to the ticket, as FR-075 does. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:234-235; spec/functional/FR-062-implement-checked-family-contract.md:48-54, 239 |
| FND-002 | medium | FR-075 lists the builder item's members (request index, occurrence key, node, kind, extent classification, candidate outcome) but does not name the item type. The existing `RequestItem` (qsl-route/src/request.rs:98-104) has no occurrence key and no candidate outcome, and `RequestWriter::item` takes only a node. The spec does not say whether the builder extends `RequestItem` or wraps it. More importantly, no listed member carries the application's result bound (its wrapping narrow's target) or its unbounded domains. Those are what CG generation (SR-638 FND-001) and a bounded follow-up request (SR-638 FND-005) need. Name the item object and add the members the driver actually passes on. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:114-131; qsl-route/src/request.rs:98-104, 194-197 |
| FND-003 | low | FR-062's `relationships` list only US-005, ADR-011, ADR-012, ADR-013, FR-063 and FR-065. The new "Requirement records of a value function" section depends normatively on FR-057 (claim-form table), FR-093 (application nodes, occurrences, `Coerce`) and ADR-014 §4 (extent rule). Add `depends_on` edges so the graph shows them. | spec/functional/FR-062-implement-checked-family-contract.md:1-17, 184-230 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@accbac3849a26f9f8206e655369105408b9f4a11` (fix commit `accbac38`, "QSL-266 spec: address spec review SR-636 to SR-640"). Each outcome was re-checked against the spec at that head, not taken from the commit message.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed accbac38 | `ClaimSite` is now defined in FR-062 "Claim sites" as a design name holding the checked `Location`, the result bound and the path condition. S3 pairs each site with the occurrence recorded at its own `Location`, "never by the order in which sites or occurrences were produced". Two occurrences are two regions, so two sites. The ADR-012 sketch comment points there (FR-062:205-226, 239-249; ADR-012:234-238). |
| FND-002 | fixed accbac38 | The item is now named `RequirementItem` (a design name). It carries the request index, occurrence key, node, kind, extent classification, unbounded domains (`DomainKey` and kind), result bound (a type-node `WireNodeId`) and candidate outcome (FR-075:77-80, 117-133). Whether it wraps or replaces `RequestItem` is left to the implementing ticket, which is acceptable for a design name. |
| FND-003 | fixed accbac38 | FR-062's frontmatter now has `depends_on` FR-057, FR-093 and ADR-014 (FR-062:18-23). |
