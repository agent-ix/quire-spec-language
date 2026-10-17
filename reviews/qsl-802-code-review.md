---
id: SR-11751
title: "Code review of quire-spec-language PR #911: state-machine contexts and the Context selection"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b7769a7d707df92277294378d0fec051dbc4f6a1; qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/frame.rs, qsl-replay/src/spine/request.rs, qsl-replay/src/spine/clause.rs, examples/config-version/spine.rs"
review_set: subset
---
# Code review of quire-spec-language PR #911

## Summary

Ticket: QSL-802. One pass of code-review with the rust-review lane over
`git diff origin/main...HEAD`. No build was run, by brief; the author's gate
passed EXIT=0.

Oracles: FR-106 checks 2, 5, 6.10 to 6.12, 7, 8, 9 and 11.6, the "read
contexts" definition, and FR-106-AC-41 to AC-47 and AC-49.

Examined: `read_contexts`, `read_context_entry`, `admit_contexts`,
`attach_read_context`, `check_context_closure`, `read_context`,
`admit_observations` check 2 by receiver, `check_operation` with
`OperationSource`, `admit_pre_call` and `admit_invocation_documents` check 9,
`frame::enforce_contexts`, and the replay clause request reader.

Known gaps not re-reported (recorded by the brief): no `context` CLI request
form; FR-122 machine-invariant counterexample stays `Current`; a FR-115 Frame
run on a state-machine operation needs the pre context; checks 7 and 8 with
two read contexts use the union; FR-106-AC-48 is not implemented.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A `PreCall` run of a state-machine precondition through the replay clause request can no longer be admitted. The request reader still requires `self` for a `pre` snapshot (`invalid("self", "required for pre snapshot")`) and always wraps it in `Some`. `admit_observations` check 2 now refuses `wrong_snapshot`/`wrong-observation` when a `PreCall` names a `self` for a state-machine receiver (`named_self.is_some_and(\|named\| named == state_machine)`). A request without `self` is refused by the reader, and a request with `self` is refused by admission. Before this PR, `admit_pre_call` ignored `self` for a state machine, so these runs admitted. The tc235 tests were changed from `Some(self)` to `None`, which only the in-process API can express. This is separate from the recorded missing `context` request form: it is a regression in the existing `pre` form. Fix: let the reader accept an absent `self` for a `pre` (and `current`) clause and pass `None`, so admission decides it by the receiver. Add a replay request test for a state-machine precondition. | qsl-replay/src/spine/request.rs:288-314, qsl-semantics/src/model/observation.rs:1795-1810 |

## Verdict

FIX: 1 blocking finding (FND-001, a regression on the replay request path).

Clean:
- Checks 6.10 to 6.12 follow the spec order: wrong kind, then missing member
  (`machine` before `fields`), then unknown member. Each entry is then checked
  for role mapping, a duplicate machine, set/bag fields, missing fields,
  unknown fields, and value rules under the object-field rules with a
  context path. Entry defects are kept at read and reported in walk order
  after objects and relationships.
- A `current` snapshot holding `contexts` refuses `unknown-member` before the
  entries are looked at. The replay environment path reaches this through
  `admit_population_values`.
- Read contexts are chosen by conformance to the clause's machine. A machine
  plus its specialization refuses `duplicate-member` at check 9. On side
  `post`, the pre snapshot is attached before the post one.
- Check 9 refuses a non-null `self` for a state machine and a null `self` for
  an object, both as `wrong-value-kind` naming `self`.
- Check 11.6 compares the pre and post contexts with the typed-value equality
  used for object fields. A retype is caught by the frame decision's type
  comparison. Only the postcondition and side `post` run it.
- The `InvocationContext` read on side `pre` skips `parameters`. This matches
  AC-42.
- rust-review: no new `unwrap`/`expect`/`panic` in production code, and
  internal invariants use `fault(..)`. Not blocking: the in-function
  `macro_rules! pre_call_context` would read better as a constructor
  function. The `contexts`-on-`current` refusal names `field` where the
  other 6.10 refusals name `member`.

## New findings (disposition pass 1)

Reviewed at 5f0bdccdc34d94dcd338da7f9085c482e709334e, over the delta from b7769a7d7 (excluding the merge of main).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | FR-312 still says `self` is a required member of a current and a pre clause request (`{name, observation, self, anchor}` / `{name, observation, self, parameters}`). FR-312-AC-5 says omitting a required member refuses `invalid-request` at stage `request`, naming it. The fix makes `self` optional in the reader, so a request without `self` now reaches admission. The new test `clause_run_command_refuses_a_missing_self_of_an_object_receiver_at_admission` asserts stage `admit` / `wrong-observation`, which is the opposite of what AC-5 says. It is tagged `FR-312-AC-2`, an AC about `call`/`clause`/`selection` shape that says nothing about `self`. Fix: update FR-312's request forms and AC-5 so `self` is optional (absent for a state-machine receiver, and an object receiver without it refused at FR-106 check 2), then retag the test to the updated AC. | spec/functional/FR-312-read-a-clause-run-request-and-run-it.md:50-55, spec/functional/FR-312-read-a-clause-run-request-and-run-it.md:136, tests/it/config_version_spine.rs:336-358 |
| FND-003 | low | `const CHARGES: usize = 48` in `machine_contexts.rs` is a fixture-size tally, not a check of any behaviour. `model_package` asserts the `normalize.record` charge count equals it, and no FR-106-AC-41..47/49 or TC-1045 row is about normalization charges. It had to go from 46 to 48 only because the fixture grew (the `Title` type). It passes whatever the charging is, provided someone re-counts. This is the count-pin antipattern. Fix: let this fixture call `model_package` without a charge count (for example `Option<usize>`, or a variant without the assertion), and delete `CHARGES`. | qsl-eval/tests/it/machine_contexts.rs:312, qsl-eval/tests/it/machine_contexts.rs:468 |

## New findings (disposition pass 2)

Reviewed at 6f0e24a11, over the fix commit 41e24df9d (the merges of main excluded).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The reworded FR-312-AC-5 says "either request without `self` constructs the selection with no `self`, and admission ... admits it when the receiver is a state machine". For a current request that is never true. Only an invariant takes `Current`, and FR-106-AC-46 refuses a state machine's invariant selected by `Current` as `wrong_snapshot`/`wrong-observation` (it takes `Context`). So the AC contradicts FR-106 for the current form. No test runs the state-machine branch through the request path either: the reader unit test only checks that `self` passes through. Fix: limit the "admits ... state machine" clause to the pre request (a state machine's precondition), and optionally add one request-path test of it. Recorded, not blocking. | spec/functional/FR-312-read-a-clause-run-request-and-run-it.md:139 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 13e2c36ca, 5f0bdccdc: the reader passes `clause.self_object` through for current and pre, and admission check 2 decides `self` against the receiver |
| FND-002 | fixed | 41e24df9d: FR-312's current and pre request forms make `self` optional and send a missing object `self` to FR-106 check 2; AC-5 and TC-891 step 5 are reworded; the test is retagged `FR-312-AC-5` (reviewed at 6f0e24a11) |
| FND-003 | fixed | 41e24df9d: `CHARGES` deleted; the fixture calls `model_package_unpinned` (reviewed at 6f0e24a11) |
