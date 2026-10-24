---
id: SR-16622
title: "Code and Rust review of QSL-1041 string-edge row clearance"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1426 (branch qsl1041-string-edge, head 313568eb3); qsl-package/src/checked_v2.rs, qsl-package/src/checked_v2/timed_model.rs, qsl-package/src/emit.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/mod.rs"
review_set: subset
---

## Summary

Ticket: QSL-1041. Review pass (round 0) at frozen head 313568eb3 against
origin/main merge base. Methods: code-review with the rust-review lane folded
in. gap-analysis skipped: the diff is a behaviour-preserving refactor with no
spec, test or requirement-surface change (FR-064's scanner and its ACs are
untouched); spec-review skipped: no `spec/**` or `plan/**` change.

Gates run in an isolated worktree and target dir: `cargo run --locked -p xtask
-- string-edge` reports 0 rows at head (14 rows at origin/main, exactly the 14
sites the PR names); `cargo fmt --all -- --check` clean; `cargo clippy --locked
-p qsl-package -p qsl-semantics --all-targets -- -D warnings` clean; `cargo test
--locked -p qsl-package -p qsl-semantics` passes.

Behaviour preservation was checked at all 14 original sites: each helper body
is the exact expression it replaced (`node_tag == "timed_model"`,
`semantic_form == "timed_model" | "clock_place" | "clock_reset" |
"clock_constraint"`, `contains("::")`), with negations carried over at
project_place/reset/constraint, emit.rs:1197 and lowering/model.rs:551.
Equality stays equality and `contains` stays `contains`, so empty strings and
partial matches behave as before. No trace tags are required: no test changed.

## Verdict

**CHANGES REQUESTED.** Behaviour is preserved and the scan is at 0, but in
qsl-package the marker relocates the string dispatch instead of removing it:
IR's quire-contract-model (locked rev 1e177e0) already provides the closed
typed vocabulary (`CheckedNodeTag::TimedModel`, `TimedModelForm::{Model,
Place, Reset, Constraint}`, `CheckedNodeKind::decode`), and sibling modules
`checked_v2/clause_root.rs` and `checked_v2/hybrid.rs` already decode through
it. The PR's local `ClockForm` enum re-types that vocabulary's wire strings.
Fixing FND-001 in this PR is about the same size as the current change and
also removes the need for FND-002's visibility change. FND-003 and FND-004
are low; FND-004 can be deferred to a follow-up ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Local ClockForm/has_form/is_timed_model_node re-type IR's closed TimedModelForm/CheckedNodeTag vocabulary and compare enum-to-string per call, so emit.rs and the projections still branch on a wire string; use CheckedNodeTag::from_wire / TimedModelForm::from_wire / CheckedNodeKind::decode as clause_root.rs and hybrid.rs do | qsl-package/src/checked_v2/timed_model.rs:730-756 |
| FND-002 | low | mod timed_model widened to pub(crate) only to export one helper; a pub(crate) re-export (or FND-001's fix, which needs no import) keeps the module private | qsl-package/src/checked_v2.rs:103 |
| FND-003 | low | New check::lowering::is_qualified_name(&str) is a third copy of the marked is_qualified helper and shares its name with value::semantic_node::is_qualified_name(&[String]), which means one-or-more valid segments; placed after the test module at file end | qsl-semantics/src/check/lowering.rs:4841-4845 |
| FND-004 | low | Checker refusals at check/mod.rs:990 and 1027 still gate on source-name spelling outside an ADR-012 section 9 edge; the typed fix is a segmented NameForm from qsl-forms, out of scope here and best deferred to a follow-up ticket | qsl-semantics/src/check/mod.rs:990 |
