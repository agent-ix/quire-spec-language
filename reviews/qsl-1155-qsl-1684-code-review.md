---
id: SR-1684001
title: "code-review (rust-review lane) of the quire-exact f4d9c97f pin: Map and BoundedQuantity arms (quire-spec-language PR 1684)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; Cargo.lock, qsl-semantics/src/check/**, qsl-semantics/src/family/requirements.rs, qsl-semantics/src/model/observation/document.rs, qsl-eval/src/simulation/model/**, qsl-eval/src/smt/layout.rs, qsl-replay/src/{execute,result/wire,spine}**"
review_set: subset
evaluated_revision: "6846a4ecedc9a27c3f6937d670742950d7dbcc03"
relationships:
  - target: ix://agent-ix/quire-exact/FR-386
    type: references
  - target: ix://agent-ix/quire-exact/FR-387
    type: references
  - target: ix://agent-ix/quire-semantic-value/FR-127
    type: references
---

# Code review of the quire-exact f4d9c97f pin (quire-spec-language PR 1684)

## Summary

Ticket: QSL-1155 (carrier; no dedicated ticket exists for the pin). One code-review pass with the
rust-review lane folded in, plus a spec-conformance read of the new arms against quire-exact FR-386
and FR-387 and quire-semantic-value FR-127, over `git diff origin/main...HEAD` at head
6846a4ecedc9a27c3f6937d670742950d7dbcc03 (base 446a6e9f; main has since moved, the merge is textually
clean).

What is right: every touched `ValueType` match is exhaustive with no `_` and no `#[allow]`; Map is
descended for the reference, float, enum, footprint and domain walks that the PR touched and is
refused with a typed cause (`WrongValueKind`, `TypeMismatch`, `OperatorIneligible`,
`missing_wire_form`) where a value or checked node would be needed; BoundedQuantity is a quantity leaf
for family encoding, equality family and domain classification and is never given a native binding
(`argument_value` falls to `wrong()`, the type node refuses `TypeMismatch`), so no bound is skipped
because no value is admitted; the family encoding gives Map and BoundedQuantity their own tags and
writes bound presence (so `BoundedQuantity [None,None]` differs from `Quantity`, as FR-387 requires);
`MalformedBoundedQuantityType` joins the causes collapsed to `TypeMismatch`;
`BoundedSuppliedAdmissionError::QuantityBoundSize` and `SuppliedNumericOperand::QuantityBound` are
named nowhere in QSL (grep). Nothing panics or drops a value. QSL builds neither type anywhere in
source yet (grep `MapType`, `BoundedQuantityType`: only the two new tests), so every arm is
defensive today.

Dependency hygiene: `Cargo.lock` changes exactly two `source` lines (quire-exact f6d62c09 to
f4d9c97f, which is origin/main of quire-exact with Map 123925b and BoundedQuantity f4d9c97; QSV 2cab5d0b
to 68a6b45c, origin/main of QSV with #70); one `quire-exact` package remains. The manifests track
`branch = "main"` by the repo's one-copy-deps convention (Cargo.toml lines 101-103: "branch = main, no
rev"); only the FCD crates are `rev`-pinned (qsl-semantics/Cargo.toml) because FCD is not a shared
dependency. A lock-only bump therefore follows convention and needs no rev pin.

Gates run at the head (one cargo job at a time, `CARGO_INCREMENTAL=0`): `cargo fmt --check` clean;
`cargo clippy --locked --workspace --all-targets -- -D warnings` clean; `cargo test --test it layer`
14 passed; `cargo test -p qsl-semantics` 791 lib + 522 it passed; qsl-eval and qsl-replay full suites passed
language-boundary guard `pass` on the PR checks (CI). QSpec main 000cd943 conformance:
qsl-semantics 7 of 7 vector checks match (25 operation vectors, 2 enum vectors, 2 compound-unit
vectors, 18 model catalog rows, 20 catalog rows, 7 accepted and 12 refused selection vectors);
qsl-package `conformance_*`: 2 of 15 pass, 13 fail with `MalformedWire` at
`/semantic_graph/nodes/N/owner/kind`, the known QSpec #461 owner_model break. PR 1684 touches no
qsl-package file and changes no IR pin, so it neither causes nor cures that red.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Two wildcard `ValueType` walks in the refinement checker were not given a Map arm, against the PR's "every site" claim. `holds_reference` ends in `_ => Ok(false)`, so `nests_reference` reports that a `Map<Text, Reference<T>>` holds no reference and the FR-135 "refinement feature the build does not check" guard is bypassed; `lift` ends in `other => other.clone()`, so a reference inside a map value is not lifted by the object map (ADR-020 RM-3) and `conforms` then compares it unlifted. Every other reference walk in the PR (`Typer::holds_references`, `footprint::referenced_types`, `observation::holds_references`, replay `holds_reference`) descends the map, so this one is inconsistent and fails open. Add `ValueType::Map(map)` to both (and list BoundedQuantity explicitly). | qsl-semantics/src/check/refinement.rs:1381-1403,1409-1424 | implementation-bug-despite-evidence |
| FND-002 | medium | The behaviours that the PR introduces in qsl-semantics are untested. The only new tests are `value_type_source` and `argument_value`; nothing pins that equality over a Map is refused `OperatorIneligible` (`lowering.rs` equality family), that `type_node` refuses both types `TypeMismatch`, that `classify_domains` classifies BoundedQuantity as `DomainKind::Quantity` and descends a Map (and does not loop on a recursive composite through a Map), that `encode_value_type` gives Map, Collection and Option different encodings and bound presence changes the BoundedQuantity encoding, or that the float scan sees a float behind a Map. Because the matches are exhaustive, deleting an arm is a compile error, but a wrong arm body (for example Map classified as a leaf, or BoundedQuantity encoded as `quantity`) passes the suite. `requirements.rs` and `family.rs` already have unit tests that build `ValueType::Quantity`, so a case per arm is cheap. | qsl-semantics/src/check/lowering.rs:4618-4629; qsl-semantics/src/family/requirements.rs:363-366; qsl-semantics/src/check/family.rs:327-340,371-376 | correct-requirement-no-evidence |
| FND-003 | medium | `#[trace("TC-927")]` cites an id that does not exist in QSL: `spec/test-cases/` has no TC-927 (the highest neighbours are TC-920 and TC-921 near that range, TC-927 is the quire-semantic-value test case for FR-127). The two doc comments and the attribute therefore bind to a foreign repository's id, so `quire matrix`/`quire trace` over QSL cannot resolve it and the tests satisfy no QSL criterion. Either add a QSL test case (and the FR/AC the arms satisfy) and trace to it, or trace the existing QSL requirement (for example the FR-154/FR-230-AC-26 consumer) and cite QSV TC-927 in prose only. | qsl-replay/src/spine.rs:1931-1932; qsl-replay/src/spine/call/tests.rs:2298-2301 | correct-requirement-no-evidence |
| FND-004 | low | The `value_type_source` test asserts only `starts_with("BoundedQuantity[")` for the bounded quantity and never exercises the bounded Map spelling (`Map<Text, V>[min, max]`), the one new branch with a format. Assert the full string for a bounded quantity and add a bounded map. | qsl-replay/src/spine.rs:1931-1948 | correct-requirement-no-evidence |
| FND-005 | low | BoundedQuantity is not a Quantity at the remaining wildcard walks, against the PR text "treated as Quantity for family, equality and domain classification" and FR-127 item 6 (a bounded value is read as `Quantity<u>` for comparison and equality): `hyper_typing::Ty::of` makes it `Ty::Value(..)` (compares only to an equal bounded type), `is_exact_numeric` and `same_dimension` in `temporal_clause/condition.rs` refuse it as a threshold, `probabilistic/model.rs` refuses it as a reward type, and `zero_of`/`RewardValue` treat it as an integer or none. All are conservative refusals, not misclassification, and unreachable until a source form builds the type; record the decision (or the follow-up ticket) rather than leaving it implicit. | qsl-semantics/src/check/hyper_typing.rs:79-90; qsl-semantics/src/check/temporal_clause/condition.rs:238,318-341; qsl-semantics/src/check/probabilistic/model.rs:1031-1035 | implementation-bug-despite-evidence |
| FND-006 | low | `witness_type` maps BoundedQuantity to `WitnessValueType::Canonical` (as Quantity) but Map to `WrongValueKind` at once, while the PR says both are refused `WrongValueKind`. The BoundedQuantity refusal happens later, in `argument_value`, after the canonical witness text is parsed; the end result is the same typed refusal, but a witness text is decoded for a parameter that can never bind. Map it to the same early refusal as Map, or state why the late one is intended. | qsl-replay/src/execute.rs:2085-2098 | correct-requirement-no-evidence |

## Review method

Skills applied: dev-team:reviewer (this role), dev-tools:code-review and dev-tools:rust-review
(single lane, this file), with the spec-conformance read of quire-exact FR-386/387 and QSV FR-127
folded in; quoin gap-analysis was not run separately (production change is a dependency bump plus
exhaustiveness arms; no new requirement or tagged-criterion inventory to compute) and no
Jev-backed analysis was run. Method: full `git diff origin/main...HEAD` read; every changed arm
compared with the FR text; `git grep` of every file naming `ValueType::Collection`/`Option` without
`ValueType::Map` (about 33 files, most of them `ir::ValueType`, a different enum) and of all
`ValueType::Quantity` sites to find missed or wildcard walks; cycle-escape (FR-127 item 4) is a QSV
walk and QSL has none of its own (QSL's `RecursionEdges` is form-based, `ValueType::Composite`
recursion in `classify_domains`/`composite_domain` is by open-composite stack and is unchanged by
the Map arm); Map over floats is not refused anywhere in QSL (only descended by the
`ieee_operation` scan); mutation reasoning for the two tests: `argument_value_refuses_map_and_bounded_quantity_parameters`
fails if either type is bound (`Ok`) or panics, and `map_and_bounded_quantity_types_have_a_source_spelling`
fails if the spelling changes, so both are real, but neither covers the semantics arms (FND-002).

## Verdict

CHANGES REQUESTED, non-blocking in risk but one fail-open guard to close: FND-001 should be fixed
in this PR (two arms), FND-003 needs a resolvable trace id, FND-002 is a request for per-arm unit
tests, FND-004 to FND-006 are low. Mergeability against current main: textually clean (no
conflict). No security or panic concern; nothing in QSL can construct either type yet.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: `holds_reference` and `lift` descend a Map (lift rebuilds `MapType` with the lifted values and the same bound) and both wildcards became explicit variant lists; check.rs reference/event walks, `contains_population` and `widen_range` also gained Map arms. Mutating either arm (lift returns the values unchanged, holds_reference returns false) fails `a_reference_in_a_map_value_is_a_nested_reference` and `lifting_reaches_a_reference_inside_a_map_value`. |
| FND-002 | fixed | Round 1: exact-figure tests for the family encoding of Map and BoundedQuantity, `type_node` refusal (`TypeMismatch`), Map equality `OperatorIneligible` and BoundedQuantity equality equal to Quantity, `classify_domains` for both (TC-437, FR-097-AC-2), and `holds_float` through a Map (the float scan was factored out as a function). |
| FND-003 | fixed | Round 1: the call test traces TC-1053 / FR-367-AC-4 (both exist, AC-4 is the WrongValueKind criterion); the spelling test carries no trace and says why. |
| FND-004 | fixed | Round 1: exact strings asserted for the unbounded and bounded Map and for `BoundedQuantity[unit]`. |
| FND-005 | fixed | Round 1: `Ty::of` reads a BoundedQuantity as `Quantity(unit)`; the threshold and reward refusals are documented as deliberate conservative refusals (QSL-1155). `Ty::of` has no direct test (nit, not blocking). |
| FND-006 | fixed | Round 1: `witness_type` refuses a BoundedQuantity `WrongValueKind` early, like a Map; the deciding-element wire encoder also refuses both types as unsupported. |

Round 1 verdict: READY at 16edcb0b0 plus this disposition commit. fmt clean; clippy `-D warnings` clean; `--test it layer` passes; qsl-semantics, qsl-eval and qsl-replay full suites pass; QSpec main 60383a35 conformance: qsl-semantics vectors 7 of 7 match, qsl-package conformance still 13 of 15 red from the QSpec #461 owner_model break, unchanged by this PR.
