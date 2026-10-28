---
id: SR-1723001
title: "code review (with rust-review) of PR #1723 (QSL-1181 slice 3): row-empty protocol subjects in V2"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1723 git diff origin/qsl-1181b-model-catalog...0c943010a (7 files); qsl-package/src/emit.rs, qsl-semantics/src/check/{mod.rs,refinement.rs,refinement/internal_list.rs} and tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1723001: code review of PR #1723 (QSL-1181 slice 3)

## Summary

Ticket: QSL-1181. Head reviewed: 0c943010a, a single commit on qsl-1181b-model-catalog (32cbba757). The scope is exactly the 7 files listed; nothing else changes.

- **`abstract` and `protocol`.** `abstract` stays the abstract `model_root`. `protocol` is `RefinementKey::Local(clause)`, and the clause is added to `dependencies`.
- **Clause lookup.** `abstract_protocol` is filled by name over the unit's protocols. That is the same lookup `resolve_protocol` uses, and that function has already refused a clause that does not act on the abstract model. A clause with no node leaves `abstract_protocol` as `None`, so the refinement is not emitted.
- **Gate.** `v2_row_empty_body` admits a protocol subject with a non-empty `abstract_internal` (FR-375-AC-12), and still refuses one for a model subject.
- **STD-524.**
  - `cend` now has empty `binder_types`.
  - `fork` and `join` now also get `Vec::new()`. A `parallel` node has no binder record, so nothing changes.
  - `effect` and `commit` keep their bare path and binder record.
  - `await` stays its `timeout`; that is already asserted by FR-147-AC-8's test and `an_await_needs_a_timeout_row_and_its_matched_event_a_row`.
- **Identity.** `abstract_internal` uses the alias-free spellings, sorted at check time. My probe (SR-1723002) shows it enters the node id.
- **Read-back.** `a_row_empty_protocol_subject_reads_back` is `#[ignore = "IR-874 ..."]`, the accepted pattern, and comment 3b349555 records when the ignore comes off. I re-measured it: IR refuses at `class_catalog/abstract/0`, as stated.

I found no code defects.

Gates at 0c943010a (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification 48373530):
- fmt: exit 0.
- qsl-semantics `refinement`/`protocol`/`catalog`: lib 177, it 98, all passing.
- qsl-package `refinement`/`catalog`/`protocol`/`owner`/`activate`/`third`: 57 pass, 2 ignored (IR-874).
- `cargo clippy -p qsl-semantics -p qsl-package --all-targets -D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean.
