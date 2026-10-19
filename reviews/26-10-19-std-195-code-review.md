---
id: SR-12930
title: "Code review (Rust lane) of STD-195 ModelLetters doc change (PR #1013)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9a8a05e9d; qsl-eval/src/model_subject.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
---
# SR-12930: Code review (Rust lane) of STD-195 ModelLetters doc change (PR #1013)

## Summary

Ticket: STD-195. PR quire-spec-language#1013 at head 9a8a05e9d. The only Rust change is the `ModelLetters::bindings` doc comment, reviewed as the rust-review lane of code-review. Gap analysis was skipped: the change has no production behaviour. The instancing implementation is QSL-873, and the repository's only `ModelLetters` impl is the test double in `qsl-analyze/tests/it/counter.rs:421`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Only `bindings`' doc changed. `ModelLetters::atom(binding, state)` is the seam where an implementation must match a state's object to the binding by population and key, not by type, and return `absent-key` for a missing or non-conforming object (FR-125's new rule). Its doc still says nothing about that, so an implementer comparing `ObjectReference`s whole (FR-107's equality) follows the doc and breaks the rule. The first line of `bindings` still says "The objects the clause's `over` parameter binds to", while the bindings are now keys with a canonical type. Update both docs once SR-12928 FND-002 is settled. | qsl-eval/src/model_subject.rs:37-52 |

## Verdict

No behavioural risk. Neither consumer of `bindings()` (`qsl-analyze/src/model_check/check.rs:217`, `qsl-replay/src/model_check/item.rs:99`) sorts, deduplicates or binary-searches the result. Both iterate in the order returned, so moving from ascending reference order to canonical order breaks no consumer. The doc's new order matches FR-125.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | `bindings()` still returns `ObjectReference`s, whose object type FR-125 no longer specifies: the canonical-member-type rule was deleted, and the doc says only that it "completes the reference and is not compared". Two implementations can return different types for one instance. The value flows into the replay `ContextBinding` (`qsl-replay/src/model_check/item.rs:99-113`), which FR-125 now says carries no type. Either name the type, or return the (population, key) pair the spec identifies an instance by. This is for QSL-873. | qsl-eval/src/model_subject.rs:37-43; qsl-replay/src/model_check/item.rs:99-113 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 01f5d081c: the `atom` doc now states matching by population and key, reading the state's own reference, and absent-key for no conforming object; the `bindings` doc says 'instances', not objects. |
