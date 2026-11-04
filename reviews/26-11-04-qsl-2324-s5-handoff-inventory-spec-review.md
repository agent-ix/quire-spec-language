---
id: SR-2324001
title: "spec-review of S5 handoff inventory spec (quire-spec-language PR 2324)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR 2324 at 8993297aa3004687158cfbd373cd3d8a24797ad7; FR-517, TC-1312, FR-519, TC-1314, FR-047, TC-131, FR-383"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-047
    type: references
---

# Spec review of S5 handoff inventory spec (quire-spec-language PR 2324)

## Summary

One pass over `git diff origin/main...HEAD` (head 8993297aa, base c37fdd5b6), read-only, planner tiering
(no mutation runs). Verdict: NOT mergeable until FND-001 to FND-003 are fixed.

Checked against the real code: `qsl_replay::qsl_inventory` (qsl-replay/src/source_inventory.rs:239-282) and
IR's `SourceInventoryOutcome` at the locked rev 35a96493 (owner, source, scope, items, observed_count,
completeness): the member set, the two row kinds, unit always 0, no expression, one source for all rows, rows
in report order (requirements, then obligations), `Complete` only for a checked unit, and
`Inventory::read` shaped like `Selection::read` (src/checked_handoff/selection.rs:274) all match.
FR-047-AC-8 matches PR 2321's test `a_parent_the_frame_forbids_is_refused_not_given_graph_semantics`
(Unchanged: Evaluate, true; ForbiddenParent: Admit, FrameViolation; Dangling: Admit, DanglingReference; truth
None). FR-517-AC-12: the author is right and the coder's claim is wrong; FR-505 line 55 makes `unit` optional,
FR-505 "Outputs" line 65 defines the unit-less form and FR-505-AC-1 admits a unit-less `Ratio`; the extreme
rationals fit the declared bounds (i64 min/max numerators, denominators 1..i64 max), and "the source reaches
both" covers reachability-based emission. QSL-1600 (Backlog) covers the three refusals of AC-18 to AC-20
exactly; STD-653 (Backlog, QSpec) and IR-1311 (Backlog) match the Dependencies text. FR-519-AC-7 uses the
repo's retired form (`**RETIRED (reason).**`, `Test (TC-xxxx), retired`; no other file cites AC-7); TC-1054 is
the fixture with `orders` and `people`, and **R** declares no population. New ids FR-517-AC-17..20 are unused
on main and no open PR touches FR-517 or TC-1312. No ticket id is in any new AC text. `quire validate` exit 0
on all 7 files.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The formal-identity rule cannot be built. FR-517 "The inventory file" sets `document` to the `source:<path>` label and the vector uses `source:unit.native`. `qsl_inventory` takes a `SourceIdentity` whose `document` is a `SourceDocumentId`; IR's `valid_identifier` (quire-contract-model identity.rs:330) requires a first ASCII letter and then only ASCII alphanumerics, `.`, `_`, `-`. `:` and `/` are rejected, so `SourceDocumentId::new("source:unit.native")` is an error and so is any path with a directory (e.g. `tests/fixtures/x.native`). The rule, AC-11, AC-17 and the vector all depend on it. The identity must be a valid identifier (for example a fixed or sanitized document id and revision 1) or the section must say how a path is mapped. | FR-517 "The inventory file" (formal identity paragraph, vector); AC-11; AC-17; IR identity.rs:330 | wrong-requirement |
| FND-002 | medium | AC-1 still says the directory has "exactly the paths of the Outputs table other than `inventory.json`" and the Outputs row says "AC-1 excludes the file until the writer writes it", while AC-11 now requires `inventory.json` in every directory unconditionally. Once the writer writes it, AC-1's "exactly" is violated; the two cannot both pass. Drop the exclusion in AC-1 and the Outputs row now that AC-11 is unconditional. | FR-517-AC-1, AC-11, Outputs table | wrong-requirement |
| FND-003 | medium | AC-16 (unchanged: "Until QSL-1558 ... a source that reads no unit-typed member is refused ...; `sum<M::Total>` refuses `UnresolvedTypeName`; `sum<Cash>` refuses `IllTyped`") and AC-13/18/19/20 are contradictory in one test: TC-1312 is one test tagged AC-1..AC-20, row 16 expects those three refusals and row 13 runs "row 16's four sources" and expects all admitted. Both cannot hold, and AC-16 carries a ticket id and "Until" with no stated end. State that AC-16 is retired (or replaced) when AC-18 to AC-20 are green, or make row 13 use its own sources. | FR-517-AC-13, AC-16, AC-18..20; TC-1312 rows 13, 16 | wrong-requirement |
| FND-004 | medium | The section says the writer builds the bytes "from the outcome `qsl_inventory` returns ... and never from the emitted package bytes", but a requirement row's `node` is a structural `CheckedNodeId` (`structural_id(key.node())`, source_inventory.rs:182) and the file requires "the wire node id string of the package". Mapping one to the other needs the package bytes, as `Selection::derive` does with `WireGraph::of_package_bytes` (selection.rs:297). Say that the node string is joined from the emitted bytes the same way, and keep "never" to the rows' other members. | FR-517 "The inventory file" first paragraph and `node` row | wrong-requirement |
| FND-005 | low | AC-10 still reads "Until FR-517-AC-1 to AC-9, AC-11 to AC-14 and FR-518-AC-1 to AC-12 are green" but the Description (line 45) now adds AC-17 to AC-20. The R table (line 243) and the paragraph at line 256 still gate `recover` on "QSL-1555" (Done) while AC-14 and Dependencies now gate on IR-1311. | FR-517 AC-10, lines 45, 243, 256 | wrong-requirement |
| FND-006 | low | Vector values: `capability_kind` `kernel.sum` is not a QSL capability kind (`Capability::to_wire` gives `value-validity`, `global-conformance`, ...) and the role of an occurrence is `declaration`/`expression`/`operand`, not `claim`. The text calls them illustrative and the test substitutes the report's, so this is not blocking; use real values so a reader can check the shape. | FR-517 vector | wrong-requirement |
| FND-007 | low | FR-047-AC-8 and TC-131 name `FrameViolation`/`DanglingReference` (enum names); the catalog codes are `frame_violation`/`dangling_reference` and the rest of the spec writes the snake_case code. Behavior matches the test. | FR-047-AC-8; TC-131 | wrong-requirement |

## Disposition

Not mergeable: fix FND-001 (identity rule), FND-002 and FND-003 (contradictions inside FR-517/TC-1312) and
FND-004; FND-005 to FND-007 may ride along.
