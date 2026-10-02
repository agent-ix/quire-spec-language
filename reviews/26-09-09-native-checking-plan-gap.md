---
id: SR-087
title: "Gap analysis of the completed native checking plan"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-005-native-checking; source cdb6560; review/handoff 5eb2332 plus final report/status changes"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-005
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-003
    type: references
---

## Summary

Plan-005's construction repair, native model, checker and qualification/handoff
tasks are complete. The selected matrix has 35/35 backed and executed cases;
reverse inspection found no implemented behavior without an owning requirement.
The full runtime/backend/Quire assignment remains incomplete.

## Verdict

**PASS** for Plan-005. This is not completion of LC02 or the full language epic.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found | - |

## Target and completion evidence

Applied the installed QUOIN gap-analysis skill at
/home/peter/.codex/plugins/cache/quoin/quoin/0.22.5/skills/gap-analysis/SKILL.md.
The target is the repository's Plan-005 bundle, spec/spec.md identifies
agent-ix/quire-spec-language, and spec/model-linking/tests.md owns TM-003.
Production/tests were reviewed at cdb6560ea26b9baad725b17ab82a315ae7c39a30;
5eb2332 adds SR-086 and the actual task log. This report and final completion
metadata do not change that tested source. Private PR #10 is pushed and ready
for review; merge is separate from this plan's reviewable-handoff deliverable.

| Task | Actual completion evidence |
| --- | --- |
| Task-011 | Done; occurrence-aware fixture source mapping and separate ownership stages at 08a4fe7, SR-083. TC-054 and the real linker/audit regressions pass in the current full run. |
| Task-008 | Done; model admission/linkage and TC-040–045 qualified at 0cd679c with SR-084/085. All model/link cases pass in the current full run. |
| Task-009 | Done; 24 checker tests at cdb6560 cover all thirteen cases. SR-086 records actual IR judgments, exact source/binding behavior, bounded proof expansion and the fixed transitive-population omission. |
| Task-010 | Done; all documented gates and code/Rust review pass. Coverage and reverse inspection are recorded here, task/matrix status is reconciled, and the private PR handoff is published. |

Every task's deliverable checkboxes are complete. The plan retains the full
downstream assignment explicitly; no runtime task was dropped to make this
milestone complete. SR-086 describes the actual first failing tests, fixed
implementation defect and final local commands. There is no claimed hosted
execution, extra agent, external semantic reviewer, Loom run or fuzz result.

## Coverage

Reconciliation: actual `quire coverage --scope
/home/peter/dev/worktrees/formalization-a-language --json`, using Quire 0.31.0
(cli 4f6ed024, engine 0.46.0@ca7362d4). Evidence is
reviews/data/native-checking/checker-coverage.json; no grep fallback was used.

- TM-003: 35/35 backed cases. All have actual execution evidence, including
  the thirteen formerly planned checker cases.
- FR-005 5/5, FR-006 5/5, FR-013 6/6, FR-014 5/5, FR-015 6/6 and FR-016 9/9
  backed criteria. FR-017 has 3/4 source-backed criteria; its remaining AC-2
  is explicitly Inspection and is qualified in SR-083.
- Rust binding census: 113 candidates, 113 tagged, 113 bound; all 62 self-named
  candidates bind. No status lies or untracked symbols.
- Actual execution: 110 default tests passed; three named private cases ignored
  in that lane, then all three passed in their selected private audit lane.
- Root rollup: 131/158 backed. This is broader than Plan-005 and does not mean
  the entire repository is implemented. FR-007/008/009/011 account for 23
  unbacked runtime/evaluation/backend/Quire criteria. The two no-source-symbol
  rows are Manual TC-010 and Inspection FR-017-AC-2; neither is a missing test.
  StR-001's two stakeholder validation criteria remain unbacked downstream.
- Eighteen existing catalog/classifier diagnostics remain disclosed, including
  heading/classifier limitations. Three IT-004 tags are extra unmatched tokens
  on otherwise fully bound private tests. They are not silently reported as
  independently reconciled integration evidence. Six existing registry duplicate
  notices occur in validation logs. None introduces an unbacked TM-003 row.

Trace presence alone does not establish correctness. The execution and actual
code/test reviews SR-083–086 supply the separate qualification. Optional semantic
gap comparison was skipped per the owner's existing decision; it was not asked
again or performed through delegated agents.

## Reverse ownership and stub inspection

Inventoried fourteen behavior groups across the implemented plan surface:

| Behavior | Owning requirement and implemented surface |
| --- | --- |
| Exact native/formal source coordinates | FR-014; formal_source.rs, retained source bridge API |
| Explicit model role admission | FR-015; native_model.rs and native_model/ |
| Deterministic model artifact identity | FR-015; native model artifact encoder |
| Model inventory/import correspondence | FR-013/015; shared linking preflight/catalog |
| Native reference and operation selection | FR-015; native linkage and selected role accessors |
| Exact authored clause/source bindings | FR-016-AC-6; checking/bindings.rs |
| Nominal contextual constraints | FR-006/016; checking/constraints.rs and types.rs |
| Lexical and observation availability | FR-016-AC-2/7; native linkage and constraint traversal |
| Presence consequences and actual IR goals | FR-006/016; checking/proof.rs and proof/facts.rs |
| Charged graph and IR expansion | FR-016-AC-8; checking/proof/graph.rs and CheckLimits/CheckUsage |
| Original AST and proof correspondence | FR-016-AC-6; CheckedPackage/CheckedClause/ProofGoal/ProofValue |
| Transitive runtime input requirements | FR-016-AC-9 with FR-007 boundary; checking/inputs.rs |
| Located checker outcomes | FR-010/016; diagnostic.rs and native-error-codes.md |
| Source-aware fixture and audit stages | FR-017; source-derived Rust fixture producer and audit orchestration |

Untraced behavior groups: 0. Source stubs: 0. Test stubs: 0. No unsigned/machine
default, textual name guessing, runtime population fabrication, hidden executable
backend or non-Rust helper was introduced. Symbolic opaque carriers and unknown
identity/reachability predicates implement the reviewed conservative proof
boundary. Future runtime requirements are explicit draft work rather than empty
functions marked complete. Model/read/proof bounds and defensive refusal paths
map to reviewed requirements; no new implicit contract was found.

The engine's production-marker count is zero out of 592 examined symbols; this
review does not relabel that as automated production trace coverage. Reverse
ownership above comes from actual code/spec inspection and prior scoped reviews.
The immutable serial implementation has no async/locking/lifecycle seam needing
Loom. No side effect or shared cache exists in the checker. No coverage threshold,
lint, test assertion or manual-only CI policy was weakened.

## Validation and claim boundary

Explicit-root Quire spec/plan/review validation passes; command results are under
reviews/data/native-checking/. SR-086 records the successful required README,
strict Rust and private audit gates for the unchanged source. Final completion
metadata and this report are validated before commit/publication.

The next work remains finite population/invocation validation, healthy and
violating reference execution with refused/incomplete outcomes, qualification
against an existing backend and Quire integration. Those require their own
concrete specify/review/plan gates and the established A/B/C ownership split.
