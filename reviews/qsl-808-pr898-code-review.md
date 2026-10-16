---
id: SR-11703
title: "Code review of quire-spec-language PR #898: process anchor refusal and single-valued navigation refusal kind"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f4bdc9a4a95084d3cebb6f93d8317a0837148f6e; qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/ir.rs, qsl-semantics/src/check/state_model.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-replay/src/spine.rs, qsl-semantics/tests/it/state_clauses.rs, qsl-semantics/tests/it/model_operations.rs, qsl-package/src/emit/tests.rs, qsl-eval/tests/it/relationship_navigation.rs"
review_set: subset
---
# Code review of quire-spec-language PR #898

## Summary

Tickets: QSL-808 (process anchor refusal) and QSL-810 (single-valued
navigation refusal kind). Code review with the Rust lane, over
`git diff origin/main...HEAD` only. No builds were run. Ticket and PR text
were read as data.

Part A (QSL-810): `NodeKind::Navigate` gains `unique`, set in
`state_model::navigation` from `end.destination()`. `destination()` picks the
far end from `near` (`Source` reads the target end, `Target` the source
end), so an inverse traversal reads the right end. The multi-valued path
still takes its collection kind from the static type built from the same
multiplicity, so the bag/set result path agrees with the refusal kind.
`single_target` now gives `Bag` for a non-unique end. Every other
`NodeKind::Navigate` match uses `..`, so nothing else needs the field, and
lowering does not emit it (it is derived from the relationship).

Part B (QSL-808): `AssemblyCause::ProcessAnchorContext` maps to
`invalid_model_binding`/`malformed-declaration`. It is raised in
`state_clauses` and `protocol_attempts` after the context resolves and
before the operation is looked up, so `Config::Flow::missing` gives this
refusal and not `missing-name`. `is_process_type` matches only
`DomainPackageRecord::Process`. A state machine, an object type, a
persistence interface and a process's subtype keep their old behaviour (the
subtype is itself a `Process` record, so it refuses too). A missing name or
a name of another meaning never reaches `object_names`, so it still refuses
`missing-name`. FR-114 says the checker refuses an attempt on a process.
Raising it in the assembler matches how attempts' missing and ambiguous
operations are already resolved there (FR-114 "exactly as FR-104's
Resolution"), and the observable code, cause and context agree with the
rows. The `qsl-replay` message arm is the only other match on
`AssemblyCause` that needed a new arm.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The refusal span covers the whole written `Config::Flow`, and the tests assert that span. FR-104 Resolution, FR-104-AC-10, FR-114-AC-5, TC-460 rows 13 to 15 and TC-513 step 5 say "at `T`" / "at `Flow`". `NameForm` has one span for `M::T`, and TC-460 row 1 puts the missing-`T` refusal at the whole `Config::Missing`, so this reads as loose spec wording rather than a code defect. Recorded, not blocking: either narrow the span to the `T` segment or amend the spec rows to `Config::Flow`. | qsl-semantics/src/check/assemble.rs:2887-2895, qsl-semantics/src/check/assemble.rs:3222-3230, qsl-semantics/tests/it/state_clauses.rs:5586-5589 |
| FND-002 | low | `is_process_type` ignores the model alias and rescans every record of every model for each clause and attempt. `model_object_types` already matches `Process` records, so it could collect the process identities once. Recorded, not blocking. | qsl-semantics/src/check/assemble.rs:3003-3016 |

## Verdict

PASS. No blocking finding. The Part A change reads `unique` from the
destination end for forward and inverse traversal, and the multi-valued path
is unchanged. Part B refuses before the operation is resolved, with the
code and cause the rows state. No panic surface, ceremony, vendoring or
compatibility layer was added. The new `match` arms are exhaustive. The
typestate, string-edge and architecture lints were not run here. Nothing in
the diff looks like a violation of them.
