---
id: SR-2329001
title: "rust-review and gap-analysis of QSL PR 2329 (tag batch A)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR 2329 head 03eed989b6d54386254e70948f60aefbbb043e34"
review_set: subset
---

# Review of PR 2329 (QSL-1671/1672/1677/1680/1681)

Mergeable: yes, with the FR-046-AC-3 tag decision and the xtask test decision below. No production change.
Gates (head trial-merged with origin/main 06729b3e3): fmt, check --locked --workspace --all-targets, clippy -D warnings default and --no-default-features, arch-lint qualified-core: all 0. Touched tests green (qsl-package protocol_view 15, qsl-forms protocol_control_forms 15, qsl-replay source_inventory, qsl-eval duplicate_preserving, qsl-semantics protocol 178, xtask seam test 1 in 347s).
Mutations: package delivery view reject [0,0] (caught protocol_view.rs:964); forms interval bounds swapped (caught by new and existing test); eval filter dedupe (caught). Not mutated: xtask test, replay test, AC-6 no-temporal test.

| Finding | Severity | Summary |
| --- | --- | --- |
| FND-001 | medium | xtask FR-507-AC-6 test 347s cold in default tests; duplicates make seam-probe |
| FND-002 | medium | FR-046-AC-3 tag covers whole AC; nominal unit U and admitted-artifact leg missing |
| FND-003 | low | FR-046-AC-3 compares Value via Debug strings |
| FND-004 | info | STOPs: FR-507-AC-7/8/11 (code feature missing), FR-362-AC-7 (spec stale), FR-218 await tag (not an FR-218 criterion) all correct |
