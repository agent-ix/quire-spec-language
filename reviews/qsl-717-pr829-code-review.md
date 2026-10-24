---
id: SR-9077
title: "Code review (with Rust lane) of quire-spec-language PR #829 for QSL-717"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@40dab6f61a2bb625206ef812459b3605b08a110b; PR #829: qsl-semantics/src/model/systems.rs, qsl-semantics/tests/it/{model_systems,model_intake,model_conformance,model_dispatch,model_population,model_normalization,tc235_business_meanings}.rs, qsl-eval/tests/it/state_model_family.rs"
review_set: subset
---
# Code review of quire-spec-language PR #829

## Summary

Ticket: QSL-717. One pass of code review with the Rust lane folded in, scoped
to the diff of `test/qualify-dispatch-identity-evidence` against
`impl/business-model-meanings`. The one production change is in
`systems::resolve_kind`: a kindless node now refuses `wrong-export` with actual
kind `none` instead of `unsupplied-producer-record`. Ten tests were added and
four were retagged.

## Verdict

One blocking finding. The `resolve_kind` change is correct. `resolve_kind` has
no production callers; its only callers are tests. The changed branch is
reached only when `actual == Kind::None`, so real-kind mismatches still go
through the existing `wrong_export` path, unchanged. The detail renders
`actual kind none`. `UnsuppliedProducerRecord` is still produced by
`unsupplied(..)` during classification, so the variant, its wire string and
its catalog mappings in `refusal.rs` are still live. Nothing in the workspace
matched on the old cause from `resolve_kind`.

FND-001 is in the same rule the PR fixes. Classification still reports a
connection end, or a port owner, that names a declared node with no systems
kind as `dangling_reference` and not as `wrong-export`/actual `none`. The
binder-request half of FR-152-AC-8 is fixed, but the connection-end half is
still wrong.

The new tests assert typed fields and the counts the QSpec rows state. R02,
R03 and R04 assert 8, 8 and 6 axis charges, and only `conformance.axis`
charges the meter in `check_operation_redefinition`. C08 and C10 assert the
zero-limit records 0/0/1. The C09 and L12 tests assert that the earlier charge
wins. Each test would fail if its rule were removed. Gate logs
`qsl717-gate-{fmt,clippy,tests-crates,root,xtask,spec}.log` all show EXIT=0.
xtask ran 182 tests and clippy ran with `--all-features -D warnings`.

Recorded only, not blocking (medium/low): the y06 retag to FR-152-AC-8 rests
on a kindless Part, not on a non-systems-meaning node. The r06 retag to
FR-151-AC-9 asserts neither the member declaration keys nor provenance. The
C09 test does not assert that admitted axis work is not refunded. No test
asserts that an FR-153-AC-8 lookup requires `absent`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `classify` reports a relationship end naming a declared non-Port node (TC-197 Y10 clause 2: `pipe`'s target replaced by `Probe`) as `dangling_reference`/`unknown-endpoint`, and a port whose owner is a declared non-Part node (TC-197 Y06 as specified: owner changed to object type `Pump`) as `dangling_reference`/`unknown-component`. FR-152-AC-8 and the kind-mapping rule require `invalid_model_binding`/`wrong-export` with actual kind `none`. The cause is that `endpoint_kinds.get`/`component_kinds.get` returning `None` is treated as "absent from the package" even when the key is a declared object type. The PR claims AC-8 added and Y06 and Y10 covered, but no test exercises either vector as written. | qsl-semantics/src/model/systems.rs:265-277, qsl-semantics/src/model/systems.rs:325-337 |
