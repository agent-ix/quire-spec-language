---
id: SR-1735001
title: "code review (with rust-review) of PR #1735 (QSL-1181 slice 3b): refinement rows in V2"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1735 git diff origin/qsl-1181c-protocol-subject...7d6a76fdf (14 files); qsl-semantics check/lowering/refinement_rows.rs, check/refinement.rs, check/refinement/rows.rs, check/mod.rs, qsl-package emit.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1735001: code review of PR #1735 (QSL-1181 slice 3b)

## Summary

Ticket: QSL-1181. Head reviewed: 7d6a76fdf, a single commit on qsl-1181c-protocol-subject (6d325215c), 14 files.

- **Binder levels (FR-341).** `row_term` mints one `parameter` node per `CheckedRow.parameters` entry at its index, and `bind_parameters` binds those same parameters to slots `0..n`, so the level equals the slot. My probe gave `attemptUpdate` a Boolean parameter `flag` and wrote `-> A::ConfigVersion::attemptUpdate(self, flag)`, `(self, not flag)` and `(self, _)`. In every case the parameter nodes are `self` at level 0 and `flag` at level 1. The argument is a reference to `flag`'s parameter node directly, or to a `unary` over it, and `_` is written `{"term":"open"}`. Nothing is mis-levelled.
- **Canonical order.**
  - Steps sort by (has no node, node digest, class spelling), so memory rows come last with `drain` before `flush`.
  - Populations and objects sort by their abstract node, fields by name, and fairness rows by (anchor, kind, granularity).
  - `step_sources` maps each canonical position to its authored row. The sidecar is (ordinal = canonical position, region = that row's own span), sorted by node id then ordinal.
- **Proof eligibility** matches FR-322's definition: no history, no `any`, no hidden field in any `objects` entry. A row-empty owner is therefore eligible and keeps its `generated` ordinal 0, as before.
- **Row locations.** A row's typing refusal now carries an `Origin::TypeDeclaration` site location. It is still placed through `span_at(&expression.spans, ..)`, which reads the path only, and qsl-semantics passes in full.
- **Unwritten rows.** A history, whole-population or `seq` row fails `rows_have_nodes`, its rows are not keyed, `v2_emittable` is false, and the declaration refuses `RefinementNotEmitted` at S4. No fault path is reachable.
- **Parameter nodes.** `row_term` mints a `parameter` node for every row parameter, used or not (`flag` exists with `(self, _)`). That is the state-clause practice. Its admission could not be read back here, because operation-row read-back is IR-1077-ignored. Not raised as a finding.

I found no code defects.

Gates at 7d6a76fdf (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification):
- fmt: exit 0.
- qsl-semantics: lib 804, it 1328, doc 30, all passing.
- qsl-package `refinement`/`catalog`/`protocol`/`rows`/`step`/`fairness`/`owner`: 84 pass, 4 ignored (IR-874 ×3, IR-1077).
- qsl-replay `refinement`: 43/43.
- `cargo clippy -p qsl-semantics -p qsl-package -p qsl-replay --all-targets -D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean.
