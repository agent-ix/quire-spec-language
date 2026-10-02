---
id: SR-342
title: "Base spec review of FR-042 and TC-121 against the producer example's claims"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md; examples/protocol-handoff/README.md"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Base checklist rerun over correction source `56c1621`, which changes FR-042
Inputs, TC-121 steps 1 and 10, and the TC-121 matrix rows. No
`type: AssuranceProfile` document exists in the local specification scope, so no
`review_selection` is enforced and the owner's selection stands: base set,
optional semantic gap extension declined, no analysis lens skills run. The one
`medium` gap recorded earlier — the unowned accepted-inventory transfer that
AC-10 depends on — is now answered in the requirement itself.

## Verdict

**CONDITIONAL** — no `high` and no `medium` finding. The example's claims remain
accurate against FR-042 and still understate rather than overstate coverage; one
`low` item is a matrix row-id duplication introduced by the AC-10 split.

## Disposition of the prior findings

| Prior | Disposition |
| --- | --- |
| FND-001 FR-042 requires an accepted inventory at the reader but no artifact owns how it reaches B | resolved. `FR-042:57-61` now requires B's integration caller to construct the accepted inventory as Rust values from independently selected original sources, admitted models, registered definitions, artifact contract and producer bytes, states that the compiler supplies those original inputs alongside its emitted bytes and reference, and denies the offered payload the right to appoint its own inventory. It names `expected.json` as a fixture inspection aid, not an interchange format or a default authority. `TC-121:109-113` carries the matching procedure for B's Rust harness. This chooses one of the three options the earlier finding listed — B authors its own selections in Rust — rather than deferring again |
| FND-002 TC-121's matrix row aggregated AC-1..AC-10 | resolved. `spec/model-linking/tests.md:111-112` splits the row and the AC-10 row names [B's IT-001](ix://agent-ix/quire-protocol/IT-001) as its prerequisite. See FND-001 below for the residue |
| FND-003 neither FR-042 nor TC-121 referenced the delivered recipe | resolved. `TC-121:37-39` links `examples/protocol-handoff/README.md` as the delivered source-to-artifact path and states plainly that it does not yet cover this case's full choreography fixture |

## Base checklist result

| Check | Result |
| --- | --- |
| ID formats (FR/TC/AC), uniqueness, sequence | pass with FND-001 — `FR-042`, `TC-121`, `FR-042-AC-1..AC-10` all well-formed; the matrix now carries two rows under one `TC-121` row id |
| Validation link integrity | pass — `quire validate --scope /home/peter/dev/worktrees/quire-language-native-handoff "spec/**/*.md"` clean, only module-registry first-wins notices; the new `ix://agent-ix/quire-protocol/IT-001` matrix link resolves |
| FR quality: description, inputs, outputs, behavior, error conditions, criteria | pass — Inputs now owns the cross-repository inventory transfer as well as the three caller-supplied revision namespaces; typed refusal vocabulary still named, not paraphrased |
| Six coverage rules | pass — every AC maps to TC-121, and AC-10's distinct open state is now expressible: it reads `backed: false` in the FR-042 criterion group (9/10) because its verification cell cites quire-protocol IT-001 alongside TC-121 |
| Cross-referencing, terminology | pass — FR-042 ↔ US-004 ↔ TC-121 ↔ IT-001 all resolve, and TC-121 step 1 now also resolves to the delivered recipe |
| Grammar corpus | 398/398 docs grammar-clean (`/tmp/quire-native-handoff-corrections-spec.log`) |

## Claims checked against the corrected requirement

- **Consumer-owned selection.** TC-121 step 10 requires B's harness to
  independently select the supplied original sources, admitted models,
  registry/contract and producer bytes, and states that neither the payload nor
  `expected.json` authorizes its own selections. The example supplies no B
  implementation and claims none, so nothing here overclaims. The new text
  defines an obligation on B, not a delivered consumer or a production sidecar
  schema.
- **Sidecar.** FR-042 Outputs still rules out "source stdout, shell commands and
  hand-edited JSON" as the producer handoff, and Inputs now explicitly demotes
  `expected.json` to an inspection aid. The README's own sidecar paragraph is
  consistent with that everywhere except its lead sentence — recorded as SR-340
  FND-001, an example-documentation fix rather than a specification defect.
- **Revision namespaces.** FR-042's separation of source-artifact labels from
  owner-derived semantic revisions is unchanged, and the implementation defect
  recorded earlier against it (SR-340 FND-002) is resolved in code, so no
  specification ambiguity remains on that axis.
- **AC-10 and the output seal.** The README retains FR-042-AC-10 and B's IT-001
  as open and now also states that the producer selects the output seal after
  emission and that the local reader check exercises no tamper controls. That is
  narrower than FR-042-AC-7's tamper obligations, which remain owed to TC-121's
  tests. No overclaim.
- **Scope of the authored workflow.** Unchanged: TC-121's positive fixture still
  describes the larger O1/O2 choreography, and TC-121 step 1 now says outright
  that the delivered recipe covers a smaller composed subject. The full positive
  fixture remains owed.

## Findings

| ID      | Severity | Summary                                                                                  | Refs                                                            |
| ------- | -------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| FND-001 | low      | The TC-121 matrix split leaves two test-case rows sharing one `TC-121` row id and status   | spec/model-linking/tests.md:111                                  |
| FND-002 | low      | The per-AC table still maps FR-042-AC-10 to TC-121 alone, beside the new prerequisite row  | spec/model-linking/tests.md:212                                  |

## Finding detail

**FND-001.** `spec/model-linking/tests.md:111-112` now holds
`TC-121 | Full compiled protocol artifact | … | FR-042-AC-1..FR-042-AC-9` and
`TC-121 | Actual compiler-to-consumer Rust handoff; requires B's IT-001 | … |
FR-042-AC-10`, both `🚧 Planned`. Coverage accepts this — 0 status lies, and the
reconciliation moved cleanly from 367/376 to 368/377 — because the engine keys
unbacked rows by `row_id` and both rows agree today. Scenario: when the first
row reaches `✅ Passed` and the second cannot, two rows report conflicting status
under the same row id, and a per-row reader (or a future `row_id` uniqueness
check) cannot tell which one a query returned. A distinct row id for the handoff
row, or a `Blocked` status marker on it, removes the ambiguity before the first
row can pass.

**FND-002.** Same item as SR-341 FND-003, recorded here because it is a matrix
edit: `spec/model-linking/tests.md:212` still reads
`| FR-042 | FR-042-AC-10 | TC-121 | 🚧 Planned |`, so the per-AC table asserts
TC-121 as AC-10's whole verification while the test-case table and FR-042's own
verification cell both name quire-protocol IT-001. One cell keeps the two tables
saying the same thing.
