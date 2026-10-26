---
id: SR-1629001
title: "Code review of quire-spec-language PR #1629: FR-230 Clock-type facet gate (580d)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@96de9b635e4b49bf0d0fc394f91fcd476f114eb3; PR #1629 diff against main (git diff origin/main...origin/qsl-580d-clock-types): qsl-semantics/src/check/{assemble,mod,timed_model}.rs; qsl-package/src/checked_v2/tests.rs and the deleted tests/timed_model_wire_port.rs; qsl-semantics/tests/it/{timed_declarations,timed_model}.rs; qsl-eval/tests/it/timed_subject.rs; qsl-forms/tests/it/time_forms.rs; qsl-analyze/tests/it/zone_search.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: reviews
---
# Code review of quire-spec-language PR #1629

## Summary

Ticket: QSL-580, remainder slice 580d. The Rust review lane is folded into
this review. Reviewed only this PR's diff against `main`.

The deleted ceremony is confirmed. `timed_model_wire_port::observe` did
nothing unless `QSL_TIMED_WIRE_BASELINE` or `QSL_TIMED_WIRE_CAPTURE` was set.
When set, it compared emitted bytes against a baseline directory, or wrote
receipt files. No Makefile, script or CI lane set either variable. The
typed round-trip and admission assertions of the calling tests are unchanged.
It tracked files and backed no behaviour, so removing it is correct.

The facet gate's new `ClockType` path, the `owner()` helper and the
`prepare_models` filter are sound. Under the facet, every source `Clock`
still refuses `TimedNotChecked`. That matches the old assembly refusal and is
replaced in #1632, so it is not a regression.

Gates: `cargo test -p qsl-semantics -p qsl-forms -p qsl-eval -p qsl-package
-p qsl-analyze --all-features` ran at this head through `locked-build.sh`,
with `QSPEC_DIR` at QSpec `origin/main` ccad669c, and passed (exit 0).

## Verdict

**CONDITIONAL.** No major bug. Two medium findings. The AC-7 facet refusal
names the wrong owning declaration for `time invariant` and `urgent`, and the
test asserts the wrong value. The FR-231-AC-2 event-position test passes the
closed reading in by hand, so no production code is tested to choose that
reading from the profile.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-230 "Timed facet gate" says the refusal retains "the owning declaration (the model for a `time` member, `time invariant` or `urgent` declaration; ...)". `TimedDeclaration::owner()` returns the model only for the `time` member. For a `time invariant` it returns the invariant's own name, and for an `urgent` declaration the declaration's own name. The FR-230-AC-7 test asserts `"Deadline"` and `"Eager"`, so it pins the behaviour the spec rules out. | qsl-semantics/src/check/assemble.rs:749-756; qsl-semantics/tests/it/timed_declarations.rs:726-737 |
| FND-002 | medium | The FR-231-AC-2 event-position half is not exercised. `under_event_position_the_execution_closes_at_its_last_position` calls `timed_trace(&timed, false)` and evaluates with a hand-built `TemporalTrace::Finite { len: 3 }`. No production code reads a clause's profile to choose `idle_tail`: `timed_trace` has no non-test caller. The clauses' `using ep` profile therefore has no effect. If the subject read event-position as the infinite idle tail, this test would still pass. | qsl-eval/tests/it/timed_subject.rs:598-655; qsl-eval/src/simulation/model/timed.rs:989-1000 |
| FND-003 | low | The gate's `clock_bearing_forms` handles a tuple element, a union payload and a collection element. FR-230's gate bullet lists all three, but no test covers them: the AC-7 test covers a parameter, a result, a record field, an alias and `Option`. | qsl-semantics/src/check/assemble.rs:757-860; qsl-semantics/tests/it/timed_declarations.rs:655-663 |
| FND-004 | low | FR-236-AC-1 is split across two tests that do not meet. The zone-search test checks the steps and delays. The `qsl-eval` test checks `TimedSystem::state_digest` on a behaviour it rebuilt. Neither checks that the counterexample's `post_state` equals the digest of the reached timed state; the doc comment states that link but no test asserts it. | qsl-analyze/tests/it/zone_search.rs:140-150; qsl-eval/tests/it/timed_subject.rs:425-470; qsl-analyze/src/zone_check/concretise.rs:246-251 |
