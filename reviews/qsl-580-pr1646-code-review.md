---
id: SR-1646001
title: "Code review of quire-spec-language PR #1646: clock places through package records (580h)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@790e70f3dbcb6d5f8b90de0e38a47f4be6df0ff0; PR #1646 diff against qsl-580g-dimension-equivalence: qsl-eval/src/simulation/model/{encode,timed}.rs; qsl-forms/src/{spans,syntax}.rs; qsl-semantics/src/check/{assemble,lowering,mod,refusal,timed_model}.rs, assemble/clock_forms.rs, lowering/model.rs, timed_model/reading.rs; qsl-semantics/src/model/state.rs; qsl-semantics/tests/it/{clock_places,timed_model,main}.rs; qsl-eval/tests/it/timed_subject.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
---
# Code review of quire-spec-language PR #1646

## Summary

Ticket: QSL-580, slice 580h, stacked on #1635. The Rust review lane is folded
into this review. I reviewed only this PR's diff against
`qsl-580g-dimension-equivalence`.

The place model is sound for the cases it admits. `clock_places` walks
package record fields from each object field and guards recursion with
`visiting`. Each `ClockField` carries its owner, its object field and its full
path. Guards, resets and reads match a place by path and by `conforms(context,
owner)`. The timed state digest writes each clock as `object`, `field`,
`path` and `value`, so two states that differ only in the value of `t.x`
against `t.y` get different digests. The record's data node drops its clock
fields. The reset cut removes the clock components from the literal and keeps
the data write.

The FR-181 typed form recurses through option, collection, record (each slot
written with types), tuple and union. Record arity is checked against the
declaration, and an absent field is written `null`.

Writing `name` as the hex checked node id is QSL FR-120's documented choice
("In these forms `name` is the ... declaration's checked node id ... as 64
lowercase hex digits"), and the code follows it. That is not a code defect.
The conflict with QSpec FR-181-AC-19's `"Shop::Item"` bytes is a spec matter,
recorded as gap-analysis SR-1646002 FND-004.

Gates: `cargo test -p qsl-semantics -p qsl-eval -p qsl-forms --all-features`
ran at 790e70f3d through `locked-build.sh` (jobs 4, `QSPEC_DIR` set) and
passed (exit 0). I also ran three local probe tests, which were not committed
and are deleted. Their results are in FND-001, FND-002 and FND-003. The
base-branch probe for FND-002 ran at `origin/qsl-580g-dimension-equivalence`.

## Verdict

**CHANGES REQUESTED.**
- FND-001 is high: a state that holds a record cannot be stepped.
- FND-002 is medium: a regression in how a clock read through a reference is refused.
- FND-003 is medium: a record literal that leaves out its clock is admitted.
- FND-004 is low: Rust and maintainability issues.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A state that holds a record value now has a key and a digest, but it cannot be stepped. Probe: over the PR's own `timer_document()` with `Call.t = Timer { n: 0 }` and a dense source, `TimedSystem::guarded_steps(initial)` returns `Err(Expansion(runtime_invariant/established-invariant-broken))`. The same world without `t` returns all of `send`, `reply`, `timeout`, `reset`, `tick`, `work` and `spawn`. Only `object_enc` was given the package's types. The step path still compares and encodes values with the type-free `value_enc`, which writes a record, tuple or union as `Unencodable`. That path covers `values_equal` (frame decision and field expansion), `text_of`, the result-domain order and the transition argument texts. So a model with a record clock place passes S3, and then every expansion stops with a broken-invariant fault. Fix: give one encoding the package's types (`value_enc`/`values_equal` included). Add a qsl-eval test that takes `send` (which resets `t.x` through the record literal) from a record-holding state and checks the post state and digest. | qsl-eval/src/simulation/model/encode.rs:317-436; qsl-eval/src/simulation/model/frames.rs:53, 815, 876, 961, 1007; qsl-eval/src/simulation/model/symbolic.rs:103 |
| FND-002 | medium | This is a regression. A clock of another object read through a reference field (`self.peer.x`, `peer: Call`) no longer refuses `invalid_timed_model`/`clock-read-outside-constraint`. `Located::new` now `continue`s on every `self_path(form, id).is_some()` before it reaches `clocks.foreign`, so any multi-step `self.` path that is not a record place is treated as reading no clock. Probe: `pre P ... { self.peer.x >= 1 ms }` at 580g refuses `Temporal(ClockReadOutsideConstraint)` at `self.peer.x`. At 580h the same input refuses `StateModel(IllTyped(TypeMismatch))` with `receiver: None`, a data-typing refusal with the wrong cause and no clock span. The same happens for `self.peer.x = self.peer.x`. Fix: skip only a data field below a clock-bearing record path (`self.t.n`) and let every other path reach `foreign`. Add a test for `self.peer.x`. | qsl-semantics/src/check/timed_model.rs:754-768 |
| FND-003 | medium | A whole-record write whose literal leaves out a clock component is admitted and resets nothing. Probe: `post Q ... send { self.t = Config::Timer { n: 3 } }` checks with no reset recorded, so `t.x` keeps its old value even though the clause writes a new `Timer` value. The checked `Timer` record now lists only `n`, so data typing cannot see the missing `x`. `record_resets` only iterates the components that are written, and the FCD IR makes a `Clock` field required (`INVALID_CLOCK_FIELD` on presence). FR-230 "Records, tuples and unions" defines the reset only for a literal "whose clock components are exact ... constants" and says nothing about one that omits a component. Either refuse `clock-write` at the literal when a clock place of its type has no component, or state the keep-the-clock meaning in FR-230 and test it. | qsl-semantics/src/check/timed_model.rs:1210-1275 |
| FND-004 | low | Rust review, smaller items. (a) `record_resets` carries `#[allow(clippy::too_many_arguments)]`. `context`, `clocks` and `written` belong in a struct, the way `ResetCut` gathers the outputs. (b) The "a place lies strictly below this path" test is written three times: `Clocks::bears`, inside `record_resets`, and `Declarations::path_kind`. (c) `clock_places()` rebuilds the whole list on every `clock_place`, `clock_field` and `path_kind` call, which happens per body node. That is quadratic in the package's records; compute it once per model. (d) `record_named` takes the first record whose last node segment matches. Unlike `object_type_named`, it does not require exactly one match. (e) The docs on `clock_places` and `ClockField` mention "union cases", but `PlaceStep` has only `Field`. (f) The `union` arm is duplicated in `value_enc_in` and `typed_value_enc`, and `value_enc_in` ignores `types` for it (`Some(_)`). A record arity mismatch is reported as "an undeclared record". | qsl-semantics/src/check/timed_model.rs:562-575, 1210-1255; qsl-semantics/src/check/lowering/model.rs:941-1050; qsl-semantics/src/check/timed_model/reading.rs:86-118; qsl-eval/src/simulation/model/encode.rs:236-300, 401-430 |
