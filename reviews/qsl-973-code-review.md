---
id: SR-12328
title: "Code review of quire-spec-language PR #1216: replay selects by the request's FR-323 replay target (QSL-973)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1af39d30e; qsl-replay/src/target.rs; qsl-replay/src/request.rs; qsl-replay/src/witness.rs; qsl-replay/src/witness/frame.rs; qsl-replay/src/witness/state_clause.rs; qsl-replay/src/execute.rs; qsl-replay/src/execute/frame.rs; qsl-replay/src/execute/state_clause.rs; qsl-replay/src/execute/value_parity.rs; qsl-replay/src/execute/operator_parity.rs; qsl-replay/src/execute/composite_parity.rs; qsl-replay/src/scalar.rs; qsl-semantics/src/check/mod.rs; tests in the diff"
review_set: subset
---
# Code review of quire-spec-language PR #1216

## Summary

Ticket: QSL-973. Covers the new `ReplayTarget` / `ReplayTargetMembers` /
`RequestTargetWire` types, FR-071 decode rule 1, FR-070 envelope
reconstruction, the target checks of `replay`, the three parity entries,
`replay_frame` and `replay_state_clause`, the new
`CheckedGraph::operation_is_ambiguous`, and the `ValueIdentity.function`
change. Includes the rust-review lane.

Measured at 1af39d30e (all probes and mutations were throwaway and restored):

- `cargo test -p qsl-replay` (lib and integration tests): 766/766 lib tests pass, and every integration binary passes.
- Refusal order (a). Probe: a frame request whose target is `function`, whose envelope target differs and whose `clause_node` is another node. It refuses `WrongReplayTarget` at `/replay/function`, so check 1 wins. TC-515 and TC-517 cover check 2 before check 3, and the target checks before the recompile.
- Decode order (a, b). Probe: all three members, and `frame_operation` plus `state_clause`, each with `arguments`, an occurrence key and an empty byte provision. Each refuses `invalid-value` at `/replay/state_clause`. A frame request carrying both extra members refuses at `/replay/arguments`. A function request carrying both flags decodes.
- Payload reads (c). `FrameCounterexample.operation` and `StateClauseCounterexample.clause` are gone. No path in the workspace reads an operation or a clause from a payload, and no path reads `selected_function`. The only leftover is a stale rustdoc (FND-005).
- Diamond (d). A fixture is cheap: about 45 lines over `config_version_domain_document_with_sub`. Add `Other` declaring `attemptUpdate`, and set `Sub.supertypes` to `[ConfigVersion, Other]`. The probe settled `AmbiguousOperation` / `ambiguous_declaration` / `ambiguous-name`. The code is right, but the test is missing (FND-003).
- Value-type object probe. The target `{Config::VersionNumber, attemptUpdate}` refuses `UnknownOperation` / `missing-name` (FND-001).
- Mutation A: run the envelope-target check before the target-kind check in `replay_state_clause`. Killed: TC-517 step 8 fails.
- Mutation B: move the `replay_state_clause` envelope-target check after the recompile. Killed.
- Mutation C: swap the `replay_frame` checks 2 and 3. Killed: TC-515 step 6 fails.
- Mutation D: let the parity entries read a `state_clause` or `frame_operation` target as the function name. It survives the whole `qsl-replay` suite (FND-002).
- `make string-edge`, `make seam-probe` and `make arch-lint-api-surface-qsl` pass. Under `--qsl-only`, arch-lint runs T12-B to T12-F and FR-100-AC-8, and skips T12-A.
- `cargo clippy -p qsl-replay -p qsl-semantics --all-targets --all-features -- -D warnings`: clean.

## Verdict

Not mergeable as is. There is one high finding: `replay_frame` gives the
wrong refusal code for a frame target whose object cannot declare operations.
The rest is sound. Decode rule 1 runs before the byte-provision rule. The
target-kind check comes first, then the envelope-target check, then the frame
identity check, and all three run before the recompile. The pointers are the
exact FR-323 strings. Clause and operation selection come only from the
request target. `ValueIdentity.function: Option<QualifiedName>` matches the
edited FR-357 text. The public API breaks are compile-loud for CG, titan and
the driver: `selected_function` is gone from the request wire, the packet and
the envelope; `FrameCounterexample.operation` and
`StateClauseCounterexample.clause` are removed;
`StateClauseReplayResult::clause` now returns `QualifiedName`; and
`ValueIdentity.function` is an `Option`. None of them is silent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A `frame_operation` whose `object` names a domain type that cannot declare operations (probe: the value type `Config::VersionNumber`) refuses `UnknownOperation` (`missing_declaration`/`missing-name`). FR-116 Behavior and QSpec FR-323 (AC-20) require `invalid_model_binding`/`malformed-declaration`. `resolve_operation` looks only in `model_objects`, which holds object types only, so the non-object type never reaches the malformed branch. | qsl-replay/src/execute/frame.rs:372-400; qsl-semantics/src/check/mod.rs:3830-3841 |
| FND-002 | medium | Nothing tests the non-function-target refusal of `replay_value_parity`, `replay_operator_parity` and `replay_composite_parity` (FR-357 Behavior: "SHALL refuse as FR-098 does"; FR-358). Mutation D, which makes these entries read the other target as a function name, survives every `qsl-replay` test. | qsl-replay/src/execute/value_parity.rs:179; qsl-replay/src/execute/operator_parity.rs:237; qsl-replay/src/execute/composite_parity.rs:65 |
| FND-003 | medium | The FR-116 `ambiguous-name` refusal (`AmbiguousOperation`, `CheckedGraph::operation_is_ambiguous`) has no test. A diamond fixture is cheap: `Other` declares `attemptUpdate`, and `Sub` gets the supertypes `[ConfigVersion, Other]`. Probed, it settles `AmbiguousOperation` / `ambiguous-name`. | qsl-replay/src/execute/frame.rs:385; qsl-semantics/src/check/mod.rs:3983 |
| FND-004 | low | `ReplayRefusal` has no `pointer()`. The `/replay/<member>` pointer of `WrongReplayTarget` exists only in the `Display` text, and a consumer can rebuild it only by calling `target.member().pointer()`. `ReplayRequestRefusal` has `pointer()`, so the two refusal types are asymmetric. | qsl-replay/src/execute.rs:286 |
| FND-005 | low | `WrongReplayTarget.entry` is a `&'static str` drawn from a closed set of six entry names, where a closed enum would be idiomatic. A rustdoc still says "`selected_function` names the enclosing function". | qsl-replay/src/execute.rs:287-293; qsl-replay/src/execute/operator_parity.rs:159 |
| FND-006 | low | The `RequestTargetWire.arguments` and `occurrence_key` bools only trigger a refusal. `ReplayRequest::to_wire` always writes `false`, so a function request decoded with either flag set does not round-trip it. This is harmless today, because a function request ignores both. | qsl-replay/src/target.rs:992-1000; qsl-replay/src/request.rs:993 |

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: a frame_operation object naming a non-object declaration refuses malformed-declaration (MalformedOperationTarget, NotAnObjectType); test an_object_that_cannot_declare_operations_is_malformed | b02604c70 |
| FND-002 | fixed: one non-function-target test per parity entry (tc_904_the_value_entry..., tc_904_the_operator_entry..., tc_907_the_composite_entry...) | b02604c70 |
| FND-003 | fixed: diamond fixture and a_name_two_supertypes_expose_is_ambiguous | b02604c70 |
| FND-004 | fixed: ReplayRefusal::pointer() | b02604c70 |
| FND-005 | fixed: ReplayEntry enum replaces the entry string; stale rustdoc corrected | b02604c70 |
| FND-006 | fixed: documented on RequestTargetWire (a function request ignores both flags; to_wire writes false) | b02604c70 |
