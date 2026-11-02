---
id: SR-2026110215394
title: "spec-review of QSL-1501 checked-package successors FR-515..FR-521 (quire-spec-language PR 2144)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-515..FR-521; spec/test-cases/TC-1310..TC-1316; FR-051, FR-052, FR-499, FR-500 amendments; spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-518
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-521
    type: references
---

# Spec review of QSL-1501 checked-package successors (quire-spec-language PR 2144)

## Summary

Ticket: QSL-1501 (tickets QSL-1544..1550). One spec-review pass over `git diff origin/main...HEAD` at head d35651704, 19 files. Sub-analyses (EARS, integrity, scope-boundary) were folded into this single pass; no separate artifact was written for them.

Checked and clean: FR-515..FR-521 and TC-1310..TC-1316 are unused on origin/main and on the diffs of all 18 other open PR heads; `quire validate` exits 0 on all 19 files; every cited AC (FR-043-AC-13, FR-052-AC-9..14, FR-053-AC-7..9, FR-513-AC-9..14, FR-050-AC-5, FR-383-AC-16/19/26, FR-390-AC-9/10, FR-493-AC-5, FR-042-AC-10/15) exists on main; the FR-499/FR-500 edits match the FR-515..FR-521 content. The 30-case manifest arithmetic holds (28 at the pin, minus 14/15/28, plus 29/30 gives the 27 of main, plus 14b/15b/31 gives 30; 18 reader plus 12 selection). Decisions 3 (new `evaluate_temporal_evidence`, FR-052 amended consistently), 4 (promote `causal_relation`, FR-383-AC-16 test retargeted by FR-519-AC-2) and 5 (R fixture with split fallback in FR-517) are specified coherently. Decision 8 (inferred stages) is guarded by the writer self-check.

Not movable by decision and awaiting Peter: cases 14, 15, 28, EvidenceRef digest members, per-binder accounting, `work.rs` limits. See FND-003 and FND-004.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Pins outside Cargo: FR-521 description names the consumer pin `9395be4` (and "30 files, 167 references"), and TC-1313 row 4 / FR-518-AC-4 make a test read identities "from the pinned writer.rs at rev 9395be4". A rev hash copied into spec and a test that reads another repo's frozen file is a pin copy; the 28 legacy identities already sit in the FR-518 table. Remove the hash and the pinned-file read; keep the table as the record. | spec/functional/FR-521-gate-the-deletions-behind-quire-protocol-on-the-checked-package-and-name-each-behavior-it-relies-on.md; spec/test-cases/TC-1313-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md | wrong-requirement |
| FND-002 | medium | FR-521 states "No table, hash or allowlist of rows is kept in code", but FR-518-AC-1 and AC-4 require a test that compares the manifest and the legacy identities to a row list, FR-518-AC-5 mutates the writer's expected table, and FR-516-AC-9 is a test over two tables that fails when a public field has no row. These are ledger tests: they protect nothing the reader, `Selection::check` and the compile-fail tests do not. Reword to behavior (every case refused as its row says) and drop the table-completeness tests. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md; spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md | wrong-requirement |
| FND-003 | medium | Row 28 (source-byte seal) gets no successor case, and rows 14/15 get by-absence cases only, as a lead default. The 28-case corpus is moved 25 of 28, not all. This is the default of ruling 1 and is stated openly, but standing rule is to spec the full feature or ask Peter; the requirement needs Peter's ruling (or a recorded successor such as an IR integrity requirement) before merge. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md | missing-requirement |
| FND-004 | medium | Further defaults that drop legacy behavior without a ruling: EvidenceRef loses `schema_digest`, `digest`, `authority_digest` and contract members (FR-516); per-binder `EvaluationReport` accounting is not carried and tc_003 report assertions move to Meter logs (FR-520); the 13-dimension `work.rs` limits are not mirrored (FR-521 row 11). Each needs Peter's ruling or a carried equivalent. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md; spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md; spec/functional/FR-521-gate-the-deletions-behind-quire-protocol-on-the-checked-package-and-name-each-behavior-it-relies-on.md | missing-requirement |
| FND-005 | medium | A refused request has two incompatible outcomes. FR-516-AC-2 says a refused request yields a result with `activation = Unavailable`, while the entry signature and text say refusal before S6a is `CallFailure::Admission`. AC-6 lists self-predecessor, other-obligation predecessor and identity mismatch as "refuse" without naming the error. Say which refusals are `Err` and which are an `Unavailable` result. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md | missing-requirement |
| FND-006 | medium | FR-516-AC-3's first clause compares requests "identical except for truth-like caller data that the type cannot carry", which cannot be constructed, so it cannot fail (AC-1's compile-fail test already covers it). FR-516-AC-4 says changing a trigger byte "changes the result_identity or is refused", a disjunction both implementations pass. State one outcome. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md | missing-requirement |
| FND-007 | medium | FR-521-AC-4 says FR-499's rows read "KEEP until FR-521 rows 2, 15 and 7 are green" and "name no other release condition", but FR-499's handoff-writer row also requires "QSL-1549 has repointed quire-protocol (ruling 4)"; and FR-500's AC-10 and AC-15 rows do not read "Successor: FR-517, FR-518" as AC-4 and TC-1316 row 4 require. The inspection AC fails against the PR's own edits. | spec/functional/FR-521-gate-the-deletions-behind-quire-protocol-on-the-checked-package-and-name-each-behavior-it-relies-on.md | wrong-requirement |
| FND-008 | medium | FR-521 rows 11 and 13 and AC-5 say the ruling is "listed for ruling (PR body)". A requirement cannot depend on a PR description, which is not part of the corpus; the decision and its default belong in the FR text (FR-518 does this; FR-521 does not). | spec/functional/FR-521-gate-the-deletions-behind-quire-protocol-on-the-checked-package-and-name-each-behavior-it-relies-on.md | missing-requirement |
| FND-009 | low | FR-517-AC-10 and FR-521 row 2 say "six `PUBLISHED_*` constants"; origin/main `handoff.rs` has five (`PUBLISHED_V1_OFFER_FILE`, `PUBLISHED_V1_SELECTION_FILE`, `PUBLISHED_OFFER_FILE`, `PUBLISHED_SELECTION_FILE`, `PUBLISHED_MUTATION_MANIFEST_FILE`), and row 14 separately lists two "already gone". Name the constants or drop the count. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md | wrong-requirement |
| FND-010 | low | FR-518 says expected values are "never derived from the reader's output in the same run", then says the table is amended with the reader's code when it differs. Rows 5-7 and the renamed-field codes are stated as unverified in the PR. AC-7 / TC-1313 row 7 search the manifest for `digest`, which the by-absence cases 14b/15b contain. State the verification status in the FR and narrow AC-7 to the expected-refusal field. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md | missing-requirement |
| FND-011 | low | FR-521-AC-3 is a QSL "Test" that builds another repository's default branch and quotes "21 lib and 54 lib-test errors" and "30 files, 167 references": snapshot counts and a cross-repo build no QSL test can run. State it as the consumer's gate (QSL-1549) with a pass condition only. | spec/functional/FR-521-gate-the-deletions-behind-quire-protocol-on-the-checked-package-and-name-each-behavior-it-relies-on.md | missing-requirement |
