---
id: SR-1953003
title: "Spec review of PR #1953 (QSL-1340): FR-411 deadlock-freedom verdict join and TC-1142 step 5"
type: SpecReview
analysis: integrity
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1953 git diff origin/main...2282c6a49: spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md, spec/test-cases/TC-1142-a-model-terminal-declaration-emits-a-v2-terminal-node.md; ticket QSL-1340"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-411
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1142
  type: reviews
---
# SR-1953003: spec review of PR #1953 (QSL-1340)

## Summary

Ticket: QSL-1340. Method: spec-review (spec-integrity-analysis, with an EARS read) over the changed text. `quire validate --scope .` passes on both files (exit 0, quire 0.36.2). The new join section is mostly precise and testable, and it agrees with the EN-1 code. Two SHALLs promise more than the code does and more than AC-5 tests.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-411 statement, use case and outputs (edited) | spec/functional/FR-411-...md:33-71 | examined |
| FR-411 Deadlock-freedom verdict join (5 bullets) | spec/functional/FR-411-...md:103-134 | examined |
| FR-411-AC-5 | spec/functional/FR-411-...md:144 | examined |
| TC-1142 purpose, step 2 (edited), step 5, expected results | spec/test-cases/TC-1142-...md:13-67 | examined |
| FR-124-AC-2 (identity claims) | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:163 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | "QSL settlement SHALL compare `terminals` with the terminal node keys of the package the run and its certificate check are over". The code does not do this for EN-6: `derived_zone_item` compares the certificate package only. AC-5 covers EN-1 only, so nothing tests the EN-6 adapter that the first bullet names. The spec is correct and the code must meet it (SR-1953001 FND-001). Then add an EN-6 clause to AC-5, or a new AC, and a TC-1142 step. | spec/functional/FR-411-...md:124-126,144 |
| FND-002 | medium | "QSL settlement SHALL compare the verdict's obligation identity 3-tuple with the request item's, and its record fields (kind, arguments and domains) with the request item's, field by field." No AC tests it, and the code does not do it as written. `derived_model_item` takes the obligation from its caller as a separate argument and never checks it against `derived`. It compares the kind and the subject identity. It does not compare arguments or domains. Either narrow the bullet to what the adapters check (kind, subject identity, terminals), or implement the comparison and give it an AC. | spec/functional/FR-411-...md:115-117 |
| FND-003 | low | The first bullet makes the join owner "the FR-281 derived-item adapters of EN-1 and EN-6 and the FR-127 settlement map". `settle_model_check` (FR-127) performs no join. The join is in the adapters and in the replay result they wrap. Two readers would place the check differently. Name the adapters and the replay refusal as the owners, and drop the FR-127 map, or say what FR-127's part is. | spec/functional/FR-411-...md:111-114 |

## Verdict

**Changes requested (one high).** The rest is sound. The O-09 explanation is consistent with FR-124-AC-2 and the code: the obligation is equal with and without the declaration, which is why the join exists. The `terminals` bullet matches `TerminalKeys` (every alias, `none` for no declaration). The refusal bullet matches `stale_dependency`/`content-mismatch` with the first differing alias and both keys. The replay bullet matches `Inconclusive(ReplayRefused)` and never `Refuted`. Moving the identity comparison out of TC-1142 step 2 and into step 5 is coherent with the retired AC-3. The EARS form is fine: each SHALL has one actor and one response, and the two If/then bullets are well formed.

## Dispositions

Disposition pass 1, head 96e889d92 (fix commit cfe66261f). `quire validate --scope .` passes on FR-411 and TC-1142 (exit 0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The EN-6 bullet now requires the run's and the check's packages, "on EN-1 and on EN-6 alike", and the code meets it. AC-6 and TC-1142 step 6 test a reduced EN-6 run alone, a certificate alone, and the matching proof. |
| FND-002 | fixed | The narrowed bullet holds against ADR-013 O-09. The 3-tuple is (node, model identity, ordinal). A derived item's node is its `model_root` anchor, which `OtherAnchor` now compares. The model identity is the anchor's model. The ordinal is assigned per (node, model), and a derived item is unique per kind and subject, so the compared kind fixes it. A derived item has no arguments or domains (unbounded, no extents), so none is left to compare. AC-6 tests `OtherAnchor`. |
| FND-003 | fixed | The owners are now the two derived adapters and the FR-128 replay refusal. The FR-127/FR-235 maps are explicitly excluded, and the FR-127 dependency edge is gone. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The refusal bullet says "If the kind or the subject differs, then the adapter SHALL refuse `OtherItem`" with no engine scope. `derived_zone_item` compares no subject identity: it checks the kind, the obligation kind, the anchor, the terminals and the clock bound. The preceding bullet correctly scopes the subject comparison to "for EN-1". Scope the refusal bullet the same way, so it does not claim an EN-6 subject check. | spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md:141-142 |
