---
id: SR-20261103112801
title: "QSL-1565 r1 code review (Rust lane) of PR #2194"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2194 head 103e438d4; qsl-eval/src/value/expression/{slot_admission.rs,evaluate.rs,mod.rs,evaluate/invariants.rs}; qsl-eval/tests/it/equality_matrix.rs; qsl-replay/src/execute/tests.rs; qsl-semantics/src/model/object_environment.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
---

## Summary

Ticket: QSL-1565. Reviewed PR #2194 at `103e438d4` (code-review with the rust-review lane folded in). The PR resolves object attributes by name, admits the whole `ObjectEnvironment` at `admit_arguments` (`slot_admission::admit_objects`), keeps the read-time admission as a defence for the protocol, temporal and clause entries, and rewrites the witness walk so a witness relaxes only the integer at its exact `index`+`steps` path.

Evidence run by this reviewer (target `~/dev/worktrees/qsl-1565-r1-target`, `CARGO_PROFILE_DEV_DEBUG=0`, via `locked-build.sh`):

- `qsl-eval --test it equality_matrix`: 49/49 pass at the PR head.
- PR merged onto `origin/main` (28e6ed112), no conflicts: `qsl-eval --test it` 930 pass, 7 fail (`tc235_business_meaning_runtime`, which also fail on `origin/main`, main-red); `qsl-eval --lib` 181/181 with `QSPEC_DIR` set to a fresh QSpec worktree; `qsl-replay --lib an_out_of_type_stored_value` passes.
- Mutations: positional slot lookup restored in `admitted_slot` is killed (`a_caller_attribute_layout_is_read_by_name...`); entry check removed is killed (6 equality_matrix tests); read-time refusal removed is killed by the lib test `the_reader_and_edge_reads_refuse_a_stored_value_of_another_type`; `widened` forced false is killed. `widened` forced true survives the whole qsl-eval suite (gap-analysis FND-002).
- Repro (scratch test, not pushed): an `ObjectEnvironment` built the way FR-106 check 6.5 builds it, with a witness for an optional (`?`) `Int[0, 9]` attribute, is refused by `admit_objects` (FND-001).
- LT-16 (`rc11_oracle::lt16_collected_sc_predecessors_still_constrain_the_sc_load`), `taskset -c 5`, `perf stat -e instructions:u`, two runs each, run outside the build lanes on a machine with load 6.8-10.6: main 202.117e9 / 202.188e9, PR merged on main 202.280e9 / 202.314e9, a delta of +0.07%, within noise. Protocol exploration goes through `evaluate_protocol_slot`, which does not call `admit_arguments`, so the explorers do not pay the environment walk.

## Verdict

**CHANGES REQUESTED.** The design is sound. The by-name lookup, the entry admission and the exact-path witnesses all work as the ticket intends, and the mutation evidence backs them. The refusal shape (`CallFailure::Admission(Refused)`, `invalid_runtime_input`/`wrong-value-kind`, locus `Member{name:"object-environment"}`) follows the TC-136 H5 precedent for `evaluate`. Three defects remain. FND-001 is a regression: a valid FR-106 post-state environment is refused. FND-002: replay turns any entry refusal of the object environment into an internal fault. FND-003: the entry walk is unbounded and cannot be cancelled. The unwrap and panic hygiene is clean (`expect` only in tests), the diff adds no `#[allow]`, and the only public addition is `ObjectEnvironment::holds_out_of_range`, which is justified. A rebase is not required. The PR merges cleanly onto current main and the focused suites agree with main there.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Valid FR-106 environments are refused. A witness for an optional (`?`) attribute, or for an optional record member, carries `steps` that begin with `OptionValue` (document.rs `Admit::Enter(PathStep::OptionValue)` on `Presence::Optional`), but the slot holds the bare value. `admits` roots the slot path at `[]` and never adds `OptionValue` for presence, so the witness never matches, and `admit_objects` and `admitted_slot` refuse `wrong-value-kind`. Main accepted these (non-empty steps relaxed the slot). Reproduced: `[]` steps admitted, `[OptionValue]` refused naming `x`. | qsl-eval/src/value/expression/slot_admission.rs:112-125,169; qsl-semantics/src/model/observation/document.rs:1868-1882,1811-1822 |
| FND-002 | medium | In replay, `convert_call_failure` maps every `CallFailure::Admission` to the internal fault `spine-function-call-has-no-protocol-slot-admission`, on the premise that the spine's call path never selects a protocol slot. The PR makes `Admission` reachable from object-environment admission on the replay `call`, `call_supplied`, bind (`admit_values`), clause and observed-window paths. Through `HoldsStop::Failed` that last path ends as a fault. So a refused environment (FND-001 or any other) becomes `TerminalValue::Failed`, not a refusal, contrary to FR-049-AC-2 ("never an internal fault") and FR-090 ("ordinary argument admission refusals retain their existing contracts"). | qsl-replay/src/spine/call.rs:879-884; qsl-replay/src/spine/bind.rs:357-361; qsl-eval/src/temporal/observed_window.rs:89-99 |
| FND-003 | medium | `admit_objects` ignores the caller's `SuppliedRun`. It never checks cancellation and charges no supplied budget (FR-277), so a large environment is walked whole, unbounded and uncancellable, on every `call` or `evaluate` entry. `ObservedWindowSource::read` calls `package.evaluate` once per position per expression, so temporal window replay now pays O(environment) per read where it used to pay for the attributes it reads. The explorers (LT-16, +0.07%) and the protocol, holds and clause entries are unaffected. | qsl-eval/src/value/expression/slot_admission.rs:37-66; qsl-eval/src/value/expression/mod.rs:626-631; qsl-eval/src/temporal/observed_window.rs:89 |

## Dispositions

- FND-001: fixed. The witness path now follows FR-106's step model: a present value of an optional attribute or record member is one `OptionValue` step down (`presence_steps`), plus `OptionValue` into an `Option` value, `Index` into a collection element and `Field` into a record member; the slot's sequence element maps to `index`. New `qsl-eval/tests/it/witnessed_environment.rs` admits a `tick` invocation through FR-106 `admit_observations` under `PostStateRange::Witness`. It asserts FR-106's own witness paths for an optional attribute, a required record's optional member, sequence elements, sequence elements' optional members and an optional record's optional member, and that every read through `evaluate` completes. With one witness removed, the read refuses at entry, naming that attribute.
- FND-002: fixed. `qsl_eval::value::object_environment_refusal` identifies the entry refusal by its locus. Replay maps it to the typed `ReplayRefusal::ObjectEnvironment(Admission(Refused))` (execute.rs) and the new `RunRefusal::ObjectEnvironment` (spine call.rs, stage `call`, code from the record). These are carried into native_result, outcome and the command output `Details::ObjectEnvironment`. Every other call admission failure on those paths is still a fault. Tests: `an_object_environment_entry_refusal_is_a_typed_run_refusal` (spine) and the restated `an_out_of_type_stored_value_refuses_where_the_equality_fault_once_reached_replay` (execute).
- FND-003: fixed. `admit_objects` polls the run's cancellation before each object, giving a supplied-value admission stop at the position after the last argument. It charges nothing to the admission budget: QSV's `SuppliedRun` exposes no admission charge for a non-argument value, and the environment's size was bounded by its own admission; this is documented on `admit_objects`. `ObservedWindowSource::read` now uses `evaluate_over_admitted_environment`. Its positions are FR-106-admitted, so it skips the entry walk and keeps the read-time check, which removes the O(environment) cost per read. The explorer/protocol-slot path is unchanged (no `admit_arguments`), so LT-16 is not affected.
