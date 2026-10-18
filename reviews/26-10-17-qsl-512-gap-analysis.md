---
id: SR-12084
title: "Gap analysis of quire-spec-language PR #938 against FR-304, FR-305 and FR-307"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@40d04ade61ce19e6e316dd35e7aacf1f9cd969d9; PR #938 diff (git diff origin/main...HEAD); spec/functional/FR-304, FR-305, FR-307; QSpec FR-450 Refusals rules 1-5"
review_set: subset
---
# Gap analysis of quire-spec-language PR #938

## Summary

Ticket: QSL-512. Planless gap analysis. Plan completion: not assessed.
`quire matrix --format tsv` on the reviewed head tags FR-304-AC-1..5,
FR-305-AC-1..3 and FR-307-AC-1..5. FR-304-AC-6/7/8 and FR-306 are untagged,
as the author recorded (out of scope for this slice, not defects). The
`FR-304-AC-6, AC-8` mention in the `abstraction.rs` module doc is not read as
coverage; the matrix leaves both untagged.

Examined and clean: FR-304-AC-1 (three bindings, keys in kind order),
AC-2 (partial relation), AC-3 (missing object type and undeclared operation,
key and package identity named), AC-4 (equal and different duplicates, both
bindings and key named), AC-5 (omitted, unknown and shared parameter, unknown
field, inherited field admitted), FR-305-AC-1 (`frame_nodes` None),
AC-2 (frame and anchor equal to the `CheckedOperationFrame`'s nodes),
AC-3 (subtype refuses, declaring type checks), FR-307-AC-1..5. Each tag on
these tests cites an AC its test fully checks, except the one in FND-002.
The oracles assert full refusal values (`assert_eq!` on the variant with its
key and entry), not just `matches!`.

## Verdict

Changes requested: two medium findings. FR-450 rule 4's receiver-collision
refusal is missing from S3, and a wrong-kind test is tagged to an AC it does
not check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-304 requires every FR-450 rule 1-5 refusal. Rule 4 includes "maps a parameter to the receiver's spelling: invalid_model_binding/malformed-declaration". `admit_binding` never compares `value.parameters` with `value.receiver`, so `FrameBinding { receiver: Parameter("store"), parameters: {n: "store", m: "limit"} }` checks. The context object and parameter `n` are then bound to one Rust input. This needs no S2 form (it is not Rust syntax and not a duplicate map entry), so it is not covered by the author's out-of-scope note. Add a `MalformedEntry` variant, the check, and a test. | qsl-semantics/src/check/abstraction.rs:472-482 |
| FND-002 | medium | `a_key_naming_a_declaration_of_another_kind_refuses_malformed_declaration` carries `#[trace("TC-798", "FR-304-AC-3")]`, but it asserts `malformed-declaration` for a wrong-kind key (FR-450 rule 2). FR-304-AC-3 is the `missing_declaration`/`missing-name` refusal. `quire matrix` binds FR-304-AC-3 to this test (export/tests.rs:512). No FR-304 AC states rule 2, so the tag should be removed (keep the test untagged under FR-304's Refusals behaviour, or ask for an AC). | qsl-package/src/export/tests.rs:510-512 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `MalformedEntry::ReceiverParameter` and the check in `admit_binding`; test `a_parameter_mapped_to_the_receivers_spelling_refuses_malformed_declaration`. |
| FND-002 | fixed | The `FR-304-AC-3` tag is dropped from the wrong-kind test; it keeps `TC-798`. |
