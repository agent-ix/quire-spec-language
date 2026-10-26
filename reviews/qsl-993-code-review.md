---
id: SR-1587003
title: "code review and rust review of QSL-993 attempt records and RecoveryView (QSL PR 1591)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7b31c224045d9d3dba6594eed4ee2f6510e55e83; qsl-eval/src/simulation/model/{application,domain,encode,footprints}.rs, qsl-eval/src/simulation/protocol.rs, qsl-eval/src/simulation/protocol/{compensation,footprint,identity,program}.rs, qsl-analyze/src/model_check/check.rs, qsl-replay/src/{outcome,proof_result}.rs, qsl-replay/src/model_check/{outcome,settle,refinement_settle}.rs, qsl-analyze/tests/it/{fill,protocol_compensation}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
---
# SR-1587003: code review and rust review of QSL-993 (QSL PR 1591)

## Summary

Ticket: QSL-993. Scope is `git diff origin/main...HEAD` on `qsl-993-attempt-records`
(head 7b31c2240), 17 files. The rust-review lane is folded into this file. Oracles: ADR-027
PS-8, ST-11, ST-12, TS-2, TS-5, CP-4, and FR-120's record form.

Both refusals are gone with no compat path: `Unsupported::UnboundAttemptRecord` and the unit
`Unsupported::RecoveryView` are deleted. `ProtocolRefusal::RecoveryView { predicate }` replaces
the second. It is mapped to `ModelCheckOutcome::RecoveryView` in `check_protocol` and
`check_protocol_liveness`, and it settles as V-8 `unsupported` with the wire cause
`recovery-view{predicate}`.

Checks run on the head: `cargo test -p qsl-analyze --test it -- protocol_` passed 98 of 98,
which includes the four new tests. Clippy and lib-test results for qsl-eval, qsl-replay and
qsl-analyze are in the report.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The attempt-type refusal is not spec'd and not tested. `ProtocolSystem::new` reuses `Unsupported::UnenumerableRecord` ("step binder of an unenumerable record type") for any template whose attempt type `enumerable_in` rejects. That covers a record with an optional, enum or nested-record field, and a tuple or union. ST-11 says `a` ranges over T's finite record domain, and TS-5 sends an unbounded root to `requires-bound`. Neither names a not-yet-implemented refusal for a bounded attempt type, and no test reaches this branch. Either spec it as a not-yet-implemented case, with a cause text that names the attempt type, and add a test, or enumerate those types. | qsl-eval/src/simulation/protocol.rs:897-908, qsl-eval/src/simulation/model/domain.rs:87-107 |
| FND-002 | low | No test pins FR-120's record form for the attempt record. The tests read it through `first_bool`, a deep search that accepts any shape holding a Boolean. A wrong `type`, `name` (the 64-hex node id) or `fields` layout from `typed_value_enc` would still pass. | qsl-eval/src/simulation/model/encode.rs:192-225, qsl-analyze/tests/it/protocol_compensation.rs:1171-1180 |
| FND-003 | low | The `PeerAttempt` clause of the FR-216-AC-7 test takes `reached[t4].steps[0]` without checking that it is a `cattempt`. At t4 the step of `Committed` is also enabled. That step binds `(ok: M::Cell)`, so it also reads and enables `cells` membership, and the assertion would pass if the order put it first. Filter on `StepKind::CAttempt`, as the `RefundAttempt` half of the same test does. | qsl-analyze/tests/it/protocol_compensation.rs:1453 |
| FND-004 | low | The write-confinement harness `assert_changes_are_written` now maps an `attempt-record` entry to the thread's `ctl`. It is only run on explorations where no thread ever holds an attempt record (`pay`, `twice`), so the new arm never runs. FT-1 confinement of the new key entry, including the commit step's removal, is never checked. Call the harness from the `attempts 2`/`not e.ok` exploration. | qsl-analyze/tests/it/protocol_compensation.rs:604-606, 637-719 |

## Verdict

No high finding: mergeable once the medium is addressed or accepted. The core is correct
against the spec:

- PS-8's four-way keep condition: `keeps_attempt`, which is `reads_local(0)`, together with
  `count < attempts`, plus clearing on the commit step.
- ST-11's per-record steps, which skip the relation on the first attempt and bind
  `earlier`/`later` on a retry.
- TS-2's `record` member.
- CP-4's commit-step `ctl` writes.

The `holds_reference` change is scoped. It narrows enabling reads only for record types with no
reference field, and that narrowing is sound because their domain does not depend on population
membership. The new composite branch in `values_of` is reachable only for attempt types:
ranged binders and record triggers are still refused through `enumerable`, which is QSL-968.

The `M::Attempt` fixture narrowing matches ADR-027 §7.1, which says the attempt record type has
one value. `touch` for `refund()` and `Cell` for `Peer` are equivalent stand-ins. The
`RefundAttempt` negative control (no `cells` read) means the receiver alone is not what
satisfies the `PeerAttempt` read assertion.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | An internal fault can be reached from source. Fix round 1 deleted the build-time refusal for an attempt type. `Space::of` returns `None` for a union and for any field type `Domain::of` cannot enumerate: String, Text, Float, Decimal, Rational, Uuid, Timestamp and `Population(Some)`. `classify_extent` (qsl-semantics family/requirements.rs) classifies a union over bounded members, and all of those scalars, as bounded, so the TS-5 pre-check lets them through. The first `cattempt` expansion then reaches `values_of(...).ok_or_else(fault("an-attempt-type-is-bounded"))`, an `established-invariant-broken` fault, where the result should be a verdict or a refusal. No test covers this. Fix: refuse such an attempt type when the system is built, with a spec'd cause, or enumerate unions (which ST-11 calls finite), and test one non-enumerable case. | qsl-eval/src/simulation/protocol/compensation.rs:453-456, qsl-eval/src/simulation/model/domain.rs:88,103, qsl-semantics/src/family/requirements.rs:395-412 |
| FND-006 | medium | The enum attempt-record form does not match FR-120. `typed_value_enc` writes `{"type":"enum","member":<variant id hex>,"rank":…}`. FR-120 says `{"type":"enum","name":…,"member":…}`, where `name` is the enum declaration's node id and `member` the declared identifier, "so a value has one FR-181 text in QSL wherever it appears". The new unit test `a_record_value_is_written_in_fr_120_s_record_form` pins the non-conforming form. This form enters transition identities and state keys. Fix: write FR-120's form, taking the names from the checked package, or leave enum attempt types out of enumeration (no AC needs them) until it can be written. | qsl-eval/src/simulation/model/encode.rs:195-213, qsl-eval/src/simulation/model/domain.rs:538-559 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 870f4b963 removes the refusal and enumerates enums, optional fields, nested records and tuples (`Space`), with unit and integration tests. The remaining non-enumerable cases now fault: see FND-005. |
| FND-002 | fixed | `a_record_value_is_written_in_fr_120_s_record_form` pins the exact JCS bytes of the record form. The enum part of that form is FND-006. |
| FND-003 | fixed | The `PeerAttempt` clause now finds the `cattempt` by `StepKind::CAttempt`. |
| FND-004 | fixed | `assert_changes_are_written` now runs in the FR-216-AC-7 loop, including the `attempts 2`/`not e.ok` exploration that holds a record. |
| FND-005 | fixed | ee414a85a enumerates unions (`Space::Union`) and lists any attempt type that `enumerable_in` rejects as an unbounded root, so the TS-5 pre-check answers `requires-bound` before exploration. `a_string_attempt_field_requires_a_bound_and_an_enum_one_is_named` covers a String field. Every production caller of `ProtocolSystem::new` runs `check_protocol_bounds` first: check.rs:673 and :808, replay.rs:626, closure.rs:356. Only tests build the system directly. |
| FND-006 | fixed | `typed_value_enc` now writes `{"type":"enum","name":<variant type node id>,"member":<declared case>}` from the package's `variant_types()`, with no `rank`. A union writes FR-120's union form. The integration test asserts the case names `fast` and `slow`, a 64-hex name and no rank. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The requires-bound cause for a non-enumerable attempt type reports the wrong domain kind and loses the field path. `protocol_domains` swaps the whole attempt type for `ValueType::Integer` at the root `<protocol>/<template>.attempt`. For `TextAttempt { s: String }` the cause says kind `integer` at path `[]`. For `WideAttempt { n: Integer }`, `classify_extent` would have named field `n` at path `[0]`, and now it does not. The root id names the right template. This follows the model's own `domains()` convention, which also lists a non-enumerable type as `Integer`, so the diagnostic is imprecise but the verdict is right. FR-101-AC-8 asks for each unbounded domain's key and kind. | qsl-eval/src/simulation/protocol.rs:611-624, qsl-eval/src/simulation/model.rs:794-812 |
