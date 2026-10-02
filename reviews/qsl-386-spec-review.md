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

## New findings (disposition pass 1)

Reviewed at 05f44bcda424d6184478e35ffd89749ad5a34a59, against QSpec FR-452,
FR-453, FR-001 and `complete-value-lock.json` on QSpec branch
`spec/wave-b-q7-adr017` (QSpec #171, open; FR-453 amended in bc860c9 after
SR-1000 ran) and the `profile` production on QSpec #174 (open).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | high | FR-345's witness result drops half of QSpec FR-453's witness check. FR-453 (Behavior "Witness", FR-453-AC-5) compiles the witness and then requests each `accepted_by` target through AD-010 capability negotiation; the layer holds only when every named target settles every item `supported`. FR-345 holds when the witness classifies `admitted`, and its Description says the gate "requests no backend". A gate built to FR-345 cannot pass FR-453-AC-5. ADR-017 RF-3 and TC-868 step 1/5 carry the same compile-only witness. Align with FR-453, or get FR-453 changed on QSpec #171 and cite it. | spec/functional/FR-345-run-the-profile-layering-refinement-gate.md:35, :108-110; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:502-505; spec/test-cases/TC-868-seeded-layering-regression-fails-the-gate.md:34-38 |
| FND-014 | medium | ADR-017 RF-6 says "a source whose header names an earlier revision label compiles under the running build's definition of that identity". Under the identity-only header (`profile <alias> = "<identity>";`, QSpec #174) a header cannot name a revision label, and FR-110-AC-5 makes one that tries a syntax error. RF-6 contradicts FR-110. Line 172 also still says a header profile resolves "against the `DefinitionLock` `root` row". | spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:570-573, :172-175 |
| FND-015 | low | FR-345's missing-item rule gives "one tool-failure result per missing item" when the corpus holds no entry or any layer/edge entry is missing. An empty corpus is missing all five witnesses and all ten edge entries, so the rule yields sixteen results, but FR-345-AC-8 and TC-868 step 6 expect exactly one naming the corpus. | spec/functional/FR-345-run-the-profile-layering-refinement-gate.md:78-82, :146 |
| FND-016 | low | FR-344's report groups and name ordering cover a corpus-level tool failure and FR-340's pair-level tool failures, but not FR-345's per-entry tool failures (malformed `entry.json`) or its missing-item tool failures (layer or edge plus missing kind). Their position and order are undefined, so byte-equal reports for a layering corpus with two such results are not specified. | spec/functional/FR-344-report-a-refinement-gate-verdict-and-exit.md:42-55; spec/functional/FR-345-run-the-profile-layering-refinement-gate.md:55-57 |
| FND-017 | medium | FR-110 "Lock rows" decides the lock's `qualification_catalog` rows for a unit whose layer closure lacks `quire.value.complete/v1` ("the `edition` row and no other"). QSpec's `package_selection.always` lists `root` (identity `quire.value.complete/v1`) and the value rule rows unconditionally, and no QSpec FR says otherwise for a state layer. QSL is normatively filling a QSpec lock-selection gap, and diverges from the lock as written. The rule belongs in QSpec (FR-001/FR-453 or the lock, on #171) and QSL should cite it. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:153-160 |
| FND-018 | low | FR-110-AC-5 and TC-490 step 4 name the removed header form (`profile v = "…" version "1" digest "sha256:…";`) in normative text. State what is instead: a header `profile` declaration ends after its identity string, and any further token is a syntax error at that token. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:175; spec/test-cases/TC-490-e3-resolves-header-profiles-against-the-definition-lock.md:36-37 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 05f44bcd (with 3cfcf8e0): FR-110 resolves a header by identity alone; the revision/digest rows, AC-4 and the root-only prose are gone; FR-341-AC-2/TC-861 use `test:unknown-profile` |
| FND-002 | fixed | 3cfcf8e0: AC-1 now expects the layer plus its `requires` closure, matching AC-7; Lock rows no longer select `root` for every unit |
| FND-003 | fixed | 3cfcf8e0: FR-110 reads layer identities, `requires` edges and admitted-form sets from QSpec's `header_selectable_layers` rows (present only on QSpec #171, open) |
| FND-004 | fixed | 3cfcf8e0: FR-343 row 4 and AC-5 make a prior `unsupported` class `unresolved (unsupported)` at any stage, matching FR-452 rule 3 |
| FND-005 | fixed | 3cfcf8e0: FR-345 gains the `entry.json` layout, malformed-entry, missing-item and empty-corpus rules, and the real-corpus local test target (AC-8, AC-9) |
| FND-006 | fixed | 3cfcf8e0: out-of-set expressions refuse `expression-form`; AC-8 tests both causes |
| FND-007 | fixed | 3cfcf8e0: FR-342 row names `Evaluate(Refuted)` with cause `UndefinedEvaluation{where, cause}` |
| FND-008 | fixed | 3cfcf8e0: FR-344 defines result groups, per-kind names and UTF-8 byte order; AC-6 covers layering |
| FND-009 | fixed | 3cfcf8e0: US-034-EX-1 says a case the prior revision refuses is not applicable |
| FND-010 | fixed | 3cfcf8e0: FR-343 and FR-345 state that QSpec FR-452/FR-453 own the results and rules, and their tables map FR-341/FR-342 classes onto them |
| FND-011 | fixed | 3cfcf8e0: FR-345 "Compile seam": the core takes the compile function; the fault wrapper lives only in `#[cfg(test)]`; TC-868 injects only through it |
| FND-012 | fixed | 3cfcf8e0: FR-340 refuses a corpus with no pair as one tool-failure result; FR-340-AC-2 tests it |
