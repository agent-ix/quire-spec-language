---
id: SR-032
title: "integrity review of LC02 evidence design"
type: SpecReview
analysis: integrity
scope: "IT-005, TM-003 and TC-020–029 against existing FR-005/006"
review_set: all
evaluated_revision: "0998e51f7051ea6996d21f6ed4db39d7e5d8f6ba"
---

## Summary

Reviewed the LC02 evidence specification using the owner's retained base plus
all seven QUOIN analyses. The owner has accepted specification e897f81 as the
internal implementation target; no external adapter or execution is invented.

## Verdict

**CONDITIONAL**: the bounded test design is reviewable, but integration/code
entry still requires the shared capability and native API/budget specification
identified below. This review does not authorize a substitute model binder.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The qualified #54 adapter/reference representation, rule-model realization and concrete native API/resource contracts are not supplied. These are explicit execution/setup prerequisites; keep the cases planned and complete the owning specifications before implementation. | IT-005 Preconditions/Inputs; TC-020–029 |
| FND-002 | low | Installed registry emits six duplicate diagnostics; functional matrix status classification still disagrees with the structural header. Preserve visible tool output and planned statuses. | data/coverage.stderr; TM-003 |

## Analysis and dispositions

ConfigVersion's real historical package and the abstract rule-model hypothesis environment remain distinct. In particular, the latter's Version domain 0..1000 cannot be silently replaced by ConfigVersion's VersionNumber domain. IT-005 requires a separately qualified realization of those hypotheses before execution. Original fixture status fields and expected evaluation values remain authored evidence; tests compare only actual static judgments. Two imports with one spelling never become first-wins resolution. Actual source/requirement numeric revision mapping must be supplied rather than cast from native opaque labels.

## Provenance and validation

Used installed QUOIN 0.20.0 specify, spec-matrix, spec-review and this analysis
skill. Actual authoring pack resolved org agent-ix and the installed IT/TC/
TestMatrix/SpecReview schemas. The owner requested all analyses and declined
only the optional gap-analysis semantic comparison. No subagent was spawned.

Quire CLI 0.31.0 / engine 0.46.0 reports 57/103 globally backed, TM-003 0/10,
and the unchanged 41/41 Rust test symbols bound. All ten new cases are planned;
there are no reported status lies. These are static observations, not a new
test run. No Cargo gate was rerun for this documentation-only packet. Raw
advisor and coverage data are retained in this review directory.

## Adoption and fixture correction review — 2026-09-08

Evaluated amendment 01a04910fdd4cdc97bf8f90aeb0b64cf1708386a using the retained
review set. Direct inspection of tests/fixtures/model-source/types/main.tsp and model-output/semantic-ir.json shows VersionNumber min 0 and max 1000, matching the abstract rule fixture's Version bounds. The earlier wording suggesting different bounds was inaccurate and IT-005 now corrects it. The declarations still have different stable identities and containing models; equal ranges alone do not establish equivalence. Original input bytes and authored judgments are unchanged. FND-001/002 and the conditional verdict remain.
