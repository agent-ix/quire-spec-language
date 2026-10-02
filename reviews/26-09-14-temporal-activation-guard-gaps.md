---
id: SR-431
title: "Temporal activation-guard selection gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "quire-spec-language#98; FR-051-AC-6; TC-139; src/protocol_artifact/checked_handoff.rs; tests/compiled_protocol_v2.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-051
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-139
    type: reviews
---

## Summary

The targeted `/gap-analysis` finds QSL #98 complete. The existing public
checked-predicate derive/read API now accepts the exact optional activation
guard retained by a checked temporal declaration without adding it to the
temporal formula's `predicate_leaves()` view. Positive, adverse and canonical
cross-wire behavior is traced to TC-139 and executes against a real admitted v2
package.

## Verdict

**PASS** — no scoped requirement, implementation, test, traceability, stub, or
reverse-trace gap remains.

## Coverage

Quire 0.32.0 reports no targeted unbacked row, status lie, no-symbol row or
unmatched trace for FR-051-AC-6 or TC-139. Repository-wide coverage remains
489/504 because of inherited work outside QSL #98; it does not obscure the
targeted binding.

## Reverse trace

| Changed behavior | Owning requirement | Executing evidence |
| --- | --- | --- |
| Exact temporal activation guard is a valid checked-predicate selection | FR-051-AC-6 | `temporal_activation_guard_is_a_distinct_checked_predicate_selection` / TC-139 |
| Guard remains outside reachable temporal formula leaves | FR-051-AC-6 | same traced test |
| Guard/formula canonical bytes cannot cross selections | FR-051-AC-2, FR-051-AC-6 | same traced test |
| Trigger and anchor handles remain invalid predicate selections | FR-051-AC-2 | same traced test |

The documentation edits state the same boundary and add no independent product
behavior. The wire schema and public function signatures are intentionally
unchanged, so no schema regeneration, digest change, migration path or new
consumer vocabulary is owed. Source stubs: 0. Test stubs: 0. Untraced changed
production behavior: 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scoped implementation or traceability gap remains. | #98; FR-051-AC-6; TC-139 |
