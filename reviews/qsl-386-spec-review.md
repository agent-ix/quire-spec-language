---
id: SR-1000
title: "Spec review of PR #583 (refinement gates: spec versioning and profile layering, STD-119 option A)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@276e08b630b8610d5928f55cfedbad9de40e0661; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/functional/FR-110-resolve-header-profile-selections-at-e3.md, spec/functional/FR-340-run-the-spec-versioning-refinement-gate-over-a-corpus.md, spec/functional/FR-341-classify-a-spine-compile-result-for-a-refinement-gate.md, spec/functional/FR-342-classify-a-clause-run-disposition-for-a-refinement-gate.md, spec/functional/FR-343-compare-prior-and-superseding-classes-per-case.md, spec/functional/FR-344-report-a-refinement-gate-verdict-and-exit.md, spec/functional/FR-345-run-the-profile-layering-refinement-gate.md, spec/test-cases/TC-490, TC-860 to TC-868, spec/usecase/US-034, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-386 (pointer on QSL-387). Review of `git diff origin/main...HEAD`
at 276e08b6 against the STD-119 option-A ruling (owner comment on STD-119),
the wave-B/wave-C owner rulings, and the QSpec counterparts FR-452 and FR-453
on QSpec branch `spec/wave-b-q7-adr017`.

Lenses applied: integrity (cross-FR consistency), QSpec consistency, EARS
phrasing, AC-to-TC coverage, caps/limits, version tracking, compat paths,
undefined-is-refuted.

What is right: the versioning gate is a behavioural comparison and compares
nothing about revision labels (FR-340, FR-343, ADR-017 RF-2); limits are
caller-configurable with published defaults and an incomplete case names the
limit, value and member (FR-340, FR-344); no depth cap; no pins, ledgers or
expected-outcome records in the corpus; every AC has a TC that tests
behaviour; all five AD-003 edges, the witness check and the proper-subset
check match FR-453 and the STD-119 ruling. `quire validate` on the 21
changed files exits 0 and `tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-110 contradicts itself and ADR-017 RF-6. Layer selection resolves a header naming `quire.value.complete/v1` (the `root` row) by identity "before the DefinitionLock table", and RF-6 now says "a header profile resolves by identity alone", yet the Profile-resolution table, AC-4, AC-5 and AC-6 still refuse `revision-mismatch`/`byte-digest-mismatch` for the root identity, the prose still says `root` is the only header-selected row, and FR-341-AC-2/TC-861 step 2 use that version refusal as their `Profile` fixture. While headers carry `version … digest …`, FR-345's parent and child units also differ outside the identity string, so every layering case is a tool failure. The header version/digest comparison is a version check (owner rule); resolve by identity alone and delete it, and give FR-341-AC-2 an unknown-identity `Profile` fixture. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:78, :118-138, :170-172; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:570; spec/functional/FR-341-classify-a-spine-compile-result-for-a-refinement-gate.md:72 |
| FND-002 | high | FR-110-AC-1 and FR-110-AC-7 cannot both pass. AC-1: a `quire.value.complete/v1` header emits exactly one `definition_selections` entry. AC-7: the same header emits that layer plus every layer it requires (`quire.state.core/v1`). The Lock rows section also makes `root` always selected, which breaks AC-7's "no layer that requires it" for a state-core header. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:153, :167, :173 |
| FND-003 | medium | FR-110 names no source for the state-core, state-queries, state-graph and complete-model layers' identities and admitted-form sets. CON-1 limits the catalog to `DefinitionLock::pinned()`, and QSpec `complete-value-lock.json` (main and spec/wave-b-q7-adr017) holds no `quire.state.*` row, so S3's layer admission and AC-7's lock rows have nothing to read. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:118-138, :161 |
| FND-004 | medium | FR-343 diverges from QSpec FR-452. A prior class `unsupported` (FR-341 rule 5; it only occurs at stage `compile`) gives `tool failure` (exit 30), but FR-452 says a case is unresolved when either revision is unsupported. A prior revision that uses an unimplemented construct is a QSL gap, not a malformed case. Treat it as `unresolved (unsupported)` at any stage, as the incomplete row already does. | spec/functional/FR-343-compare-prior-and-superseding-classes-per-case.md:47 |
| FND-005 | medium | FR-345 says it "SHALL cover exactly" five edges but gives no corpus layout (FR-340 has `pair.json`), no malformed-corpus rule, and no result when a layer has no witness or an edge has no distinguishing unit or no cases. An empty or partial corpus therefore reports success, exit 0. Unlike FR-340, it also has no "Where it runs" real corpus in the local test target. | spec/functional/FR-345-run-the-profile-layering-refinement-gate.md:40-58, :59 |
| FND-006 | low | FR-110 Layer selection refuses every out-of-set form with cause `declaration-form`. AC-8 applies that to a named-predicate call, which is an expression, and FR-341 rule 6 also expects `expression-form`. It is unclear which cause an out-of-set expression gets. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:133, :174; spec/functional/FR-341-classify-a-spine-compile-result-for-a-refinement-gate.md:55 |
| FND-007 | low | FR-342's exhaustive-match row names `Evaluate(Undefined)`, a result kind the QSL-366 ruling removes: undefined settles refuted with cause `UndefinedEvaluation`. The class `refused` is right, but the row should name the refuted outcome and its cause. | spec/functional/FR-342-classify-a-clause-run-disposition-for-a-refinement-gate.md:53 |
| FND-008 | low | FR-344 orders results "by the case's name … each as its bytes", but layer and edge results (FR-345) have no defined name or position relative to cases, and "its bytes" names no encoding. AC-3 only covers versioning cases. | spec/functional/FR-344-report-a-refinement-gate-verdict-and-exit.md:42-45 |
| FND-009 | low | US-034-EX-1 says "every other case holds", but FR-343-AC-2 and TC-864 make the pair's violating-parent case `not applicable`. | spec/usecase/US-034-catch-a-refinement-regression-across-revisions-and-profiles.md:56 |
| FND-010 | low | FR-345 restates QSpec FR-453's edge table and comparison rules, and FR-343 restates FR-452's, as normative QSL SHALLs instead of citing them. That is the drift path behind FND-004. | spec/functional/FR-345-run-the-profile-layering-refinement-gate.md:59-108; spec/functional/FR-343-compare-prior-and-superseding-classes-per-case.md:40-55 |
| FND-011 | low | The negative halves of TC-868 steps 3 and 5 (FR-345-AC-4, AC-6, AC-7) need a "test-only fault" inside the compiler under test, and no FR says where that seam lives or that production builds do not have it. | spec/test-cases/TC-868-seeded-layering-regression-fails-the-gate.md:35, :40 |
| FND-012 | low | FR-340 lets a corpus have zero pairs, which reports success, exit 0. If the real corpus is emptied, the local test target still passes (FR-340-AC-6). | spec/functional/FR-340-run-the-spec-versioning-refinement-gate-over-a-corpus.md:50-52, :108-112 |

## Verdict

Not mergeable as it stands: FND-001 and FND-002 are contradictions inside
FR-110, and FND-001 blocks the layering gate (FR-345) from running. FND-003 to
FND-005 are real gaps an implementer would trip on. The versioning half
(FR-340 to FR-344) is sound apart from FND-004, FND-008 and FND-012.
