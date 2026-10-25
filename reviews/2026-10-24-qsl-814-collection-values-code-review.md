---
id: SR-17034
title: "Code review of quire-spec-language PR 1478 (QSL-814): set, bag and ordered-set value forms"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1393f5efaeb5e9dccb9e5c8b8fe650ca2c2f2dba; PR #1478 diff against origin/main; qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/{document,encode,tree}.rs, qsl-replay/src/spine/request.rs, qsl-semantics/tests/it/{native_snapshot_values,observation_composites,state_clauses,variant_fields}.rs, qsl-semantics/tests/support/native_values.rs, qsl-eval/tests/it/machine_contexts.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-106
---
# Code review of quire-spec-language PR #1478

## Summary

Ticket: QSL-814. This is a code-review pass with the rust-review lane folded in.
It covers the full `git diff origin/main...HEAD` (11 files, +2008/-194) at
1393f5efa.

Examined:
- FR-106 "Set, bag and ordered-set values" statement, every bullet (examined)
- FR-106-AC-50, FR-106-AC-51, FR-106-AC-52 against their tagged tests (examined)
- FR-106-AC-16 constant-stack guarantee, as it applies to the new comparison path (context_only)
- Raw multiset comparison `SnapshotValue::forms_equal` (examined)
- `form_collection_value`: the duplicate earlier-index rule and set/bag grouping (examined)
- `bound_refusal`, which checks the bound before the elements (examined)
- `canonical_bytes`, the RFC 8785 order for set and bag writes (examined)
- `ValueEncoder::encode` canonical Text inside set and bag elements (examined)
- `widen_value` / `widen_range`, the Witness type-widening fix (examined)
- the tree.rs and request.rs readers (examined)

Gates run by the reviewer, one cargo job at a time, with a private CARGO_TARGET_DIR:
- The 19 touched `qsl-semantics --test it` tests pass. So do qsl-eval
  `a_context_set_field...` and `a_context_reference_closes...`, the qsl-replay
  `parameter_value_reader...` test and the document.rs unit test.
- `cargo clippy --no-deps --all-targets` on qsl-semantics, qsl-replay and
  qsl-eval reports nothing in any touched file. The only lints are in untouched
  files and already fail on main: `check/check/typing.rs:116`
  (empty_line_after_doc_comments) and `check/mod.rs:1088` (needless_borrow).
  The qsl-forms large_enum_variant lint is also pre-existing.
- No `#[allow(...)]` and no lint relaxation appear anywhere in the diff.
- `make string-edge` reports 3 unmarked occurrences, all in untouched files
  (qsl-package/src/emit.rs, qsl-replay/src/result/wire.rs), none in the diff.
  The new `collection_kind_of_tag` is marked `#[ix_trace_rs::string_edge]`.
- Mutation probes ran on a private copy (git archive):
  - Disabling `widen_value` fails `the_frame_and_the_limits_see_collections_by_value`
    at native_snapshot_values.rs:2527, so the widening fix is tested.
  - Replacing the min-later duplicate choice survives every test (FND-002).
  - A 20,000-deep nested Set compared with `==` overflows the stack, while the
    same depth of Sequence passes (FND-001).

## Verdict

Changes requested. The admission, bound-before-element order, wrong-kind
refusals, Witness index, values counting and widening fix are correct and
tested. The RFC 8785 byte order is right too: `9` sorts after `10`, and a
reference sorts key-first. No production `unwrap`, `expect`, index panic or
lossy integer conversion is introduced. One high finding: the new multiset
comparison recurses on the native stack per nesting level, which regresses
the constant-stack guarantee FR-106 gives every other form.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `forms_equal` compares set/bag items with a nested recursive call per nesting level, so the native stack grows with document nesting. Reviewer probe: a 20,000-deep `Set` raw value compared with `==` aborts with stack overflow (SIGABRT, test thread), while the same depth of `Sequence` passes. The frame check reaches this path for every field (frame.rs:695). With a recursive record type holding a set field, untrusted pre/post snapshots can abort the process. This breaks FR-106-AC-16's no-stack-depth guarantee. The matching is also O(n^2) and unmetered. Fix: compare the admitted typed values (canonical key order, `compare_keys`), or make the matching iterative. | qsl-semantics/src/model/observation.rs:1047-1116 |
| FND-002 | medium | Mutation survives: the rule that refuses at the earliest later duplicate across several duplicate groups is untested. Replacing `Some((found, _)) if found <= *later => {}` with `Some(_) => {}` passes all six duplicate tests, because each fixture has exactly one duplicate group. Add a case such as set `[1, 2, 2, 1]`, which must refuse at element 2 naming 1, not at element 3 naming 0. | qsl-semantics/src/model/observation/document.rs:2050-2066; qsl-semantics/tests/it/observation_composites.rs |
| FND-003 | low | The `cardinality_out_of_bound` record names the collection by its wire tag (`collection: "bag"`), not by the collection type FR-106 requires ("naming the collection type, the bound and the supplied count"). | qsl-semantics/src/model/observation/document.rs:2001-2014 |
| FND-004 | low | Check 6.2/6.12 refuses a set/bag/ordered-set of a population by naming the `field`, not the element type that FR-106's last bullet requires ("SHALL refuse ... naming the element type"). | qsl-semantics/src/model/observation/document.rs:1961-1972, 2385-2392, 2606-2613 |
| FND-005 | low | `canonical_bytes` silently falls back to serde_json bytes when the canonical read or write fails, and does so under `u64::MAX` limits, so any failure would quietly produce a non-RFC-8785 order. It should be an internal fault, or an infallible canonical encoder should be used. | qsl-semantics/src/model/observation/encode.rs:273-280 |

## Dispositions

Round 1. The fix landed in PR #1480 (head aa5276432, code commit 854387bf3).
#1478 had already merged to main at 19a1d3a63. The reviewer checked a git
archive of aa5276432 in a private target dir:
- The touched tests pass. `variant_fields::each_mistyped_use_of_a_variant_type_refuses_with_its_cause` fails, but that is the main NameForm baseline (#1467) and the PR does not touch that test.
- `cargo check --workspace --all-targets` is clean.
- Clippy `--no-deps` shows only the baseline lints, in untouched files.
- string-edge shows only the 3 baseline occurrences.
- The diff adds no `#[allow]`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `forms_equal` now reduces both sides to a hash-consed `structure_identity` with an explicit heap stack. Set and bag child identities are sorted, every other form keeps its order, and the length-prefixed encoding is injective. Cost is O(n log n). `SnapshotValue::collection` keeps the first item's arena, so building a chain is linear. `raw_comparison_of_deeply_nested_sets_does_not_grow_the_stack` (depth 20,000, for Set, Bag and Sequence, equal and unequal cases) passes at aa5276432. The reviewer's earlier probe aborted with a stack overflow on 1393f5efa for the same construction. |
| FND-002 | fixed | `the_first_duplicate_in_array_order_is_the_one_refused`: `[1, 2, 2, 1]` refuses at Index(2) with earlier-index 1, for Set and OrderedSet, both ways. Under the reviewer's mutation (`Some(_) => {}`) it would refuse at Index(3) naming 0, so the mutation is killed. |
| FND-003 | accepted-no-change | `collection` is the member that FR-096's kernel catalog row defines as "the collection kind", and the kernel emitters use it that way. Changing admission alone would split the vocabulary. FR-106's "naming the collection type" wording is the divergence and needs a spec follow-up. That same FR-096 row also names a single `bound` member (`[minimum, maximum]`), whereas admission writes `minimum` and `maximum`. The `kernel_fields` validator only reads evaluate-stage outcomes, never admission refusals, so no reader rejects this. |
| FND-004 | fixed | Checks 6.2 and 6.12 now add `feature = "Population"`, the member that names an unsupported element type elsewhere. No model document can declare such a field, so only the predicate unit test covers it. |
| FND-005 | fixed | `to_json` returns `Result<_, AdmissionFailure>` and `canonical_bytes` faults (`collection-element-reads-back` / `collection-element-has-a-canonical-form`), with no serde fallback. The one production caller, `write_environment_document` (write.rs:88), propagates with `?`. Callers in tests `.expect`. The workspace compiles with all targets. |
