---
id: SR-680
title: "Code and Rust review of kernel ForeignReference universes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2fc99d9d8457bc5e838a47842f33a5e5ba053c8f; quire-exact/src/outcome.rs; quire-exact/src/equality.rs; qsl-foundation/src/diagnostic.rs; qsl-eval/tests/it/collection_algebra.rs; qsl-eval/tests/it/equality_matrix.rs; qsl-eval/tests/it/model_reference_queries.rs; qsl-replay/src/spine/call/tests.rs; src/command/output.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: QSL-281. PR: quire-spec-language#479 at 2fc99d9d. Code review with the
rust-review lane, scoped to `git diff origin/main...HEAD` (8 Rust files).

Sound: `Refusal::ForeignReference { required, supplied }` stays `Copy`
(`UniverseId` is `Copy`). The only raise site is `plan_pairs`
(quire-exact/src/equality.rs:164-170), and nested pairs (option payloads,
composite slots, collection ranks) keep left/right order, so the convention
holds at every depth. Every match site is updated: `code()`, the IEEE-flags
match, `kernel_refusal_record`, and the `#[deny(clippy::wildcard_enum_match_arm)]`
map. The record is `CatalogCode::new("foreign_reference", "foreign-universe")`
with keys `required`/`supplied`, the same code, cause and keys as
`ModelRefusalCause::ForeignUniverse::catalog_fields`
(qsl-semantics/src/model/refusal.rs:683-686). No new code is added.
`UniverseId`'s `Display` (quire-exact/src/identity.rs:98-101) is `{byte:02x}`
per byte, byte-for-byte the same lowercase hex as `model::key::hex`. No
`unwrap`/`expect`/`panic!` was added outside tests.

Swap mutant, run by the reviewer: `required: r.universe(), supplied:
l.universe()` at equality.rs:167-168 fails TC-322 (both tests),
qsl-eval `e16_references_compare_identity_triple_only` and
`c10_reference_holder_sets_charge_pairs_and_refuse_foreign_universes`. The
test universes are distinct (digest 10 vs 20, u1 vs u2, 0x01 vs 0x02). A swap
inside `kernel_refusal_record` fails
`a_kernel_foreign_reference_builds_a_record_with_both_universes` and
`tc_452_step_4_outcome_mapping_covers_every_category`.

The defect is semantic. "left is required" is right for `a == b`, which has
no preferred side. But `member_equal(candidate, member)` passes the probe
first, so at both membership call sites the probed value becomes `required`.
That is the reverse of FR-096's meaning, where `required` is the universe
already in force (the binding's).

## Verdict

**Changes requested.** One medium finding; the rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `required` = left operand inverts FR-096's `required`/`supplied` meaning at both membership call sites. `member_equal_stop` calls `plan_pairs(candidate, member)`. In `Contains` (`x in c`) the probed item becomes `required` and the collection's member `supplied`. In `coalesce` the new candidate becomes `required` and the already-retained member `supplied`. FR-096's `foreign_reference` row defines `required` as the universe in force (the binding's) and `supplied` as the one presented. Failure scenario: `hx in holders`, with `holders` of universe u1 and `hx` of u2, renders `{"required": u2, "supplied": u1}`. A reader following FR-096 concludes the collection required u2. The c10 test pins this inverted order and its comment says so (collection_algebra.rs:538-548). Fix: pass `plan_pairs(member, candidate)` in `member_equal_stop` (equality is symmetric, so the pair count and result do not change), or have FR-096 state the rule explicitly (see SR-682 FND-001). | quire-exact/src/collection.rs:425; qsl-eval/src/value/expression/evaluate.rs:1220; quire-exact/src/collection.rs:387; quire-exact/src/outcome.rs:145-156; qsl-eval/tests/it/collection_algebra.rs:538-548 |
| FND-002 | low | `tc_322_foreign_reference_universes_are_not_swapped` repeats `tc_322_foreign_reference_pair_is_refused` (same fixture, same call). The first test already asserts the full variant with distinct universes, so the second adds only `assert_ne!(required, supplied)`, which the fixture guarantees. It adds no mutant-killing power. | quire-exact/src/equality.rs:356-386 |
| FND-003 | low | The TC-452 step 4 test in qsl-replay builds its expected fields with `required.to_string()`, the same `Display` the implementation uses. TC-452 names the literal `"01"` x32 / `"02"` x32. The CLI test uses literals but builds `CallRefusal::Record` directly, bypassing conversion. Failure scenario: a change to `UniverseId`'s `Display` (uppercase, a `0x` prefix, `Debug` spelling) keeps both tests green while the rendered record leaves FR-096's lowercase hex. Only the qsl-eval TC-428 test uses an independent hex oracle. | qsl-replay/src/spine/call/tests.rs:476-482; src/command/output.rs:752-774 |

## Rust review

- Panic surface: clean. New `panic!`/`unwrap` appear only in tests.
- Wildcards: `kernel_refusal_record` stays `#[deny(clippy::wildcard_enum_match_arm)]` and lists every variant; `code()` and the flags match use `{ .. }` for the new fields.
- Copy/Eq: `Refusal` keeps `Clone, Copy, Debug, Eq, Hash, PartialEq`.
- Doc comments on the variant and its fields state the convention. The convention itself is FND-001.
- No integer conversions or I/O in the change.

## Gate

`make ci` at 2fc99d9d, re-run by the reviewer with one target dir in the
worktree. Log: scratchpad/rv479/make-ci.log (SHA first line, `exit=` last). Result: exit 0. Mutant logs: scratchpad/rv479/mut-raise-swap.log, scratchpad/rv479/mut-record-swap.log (both mutants killed; tree restored clean).
