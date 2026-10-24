---
id: SR-16358
title: "Code and Rust review of QSL-1000 protocol and weak-memory tests and trace tags"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1420 head 3c685078fa7644bc453a800cceae3e7534641167 vs origin/main 566ce2bcf; qsl-analyze/tests/it/{protocol_compensation,protocol_constructs,protocol_system,release_acquire,weak_memory}.rs, qsl-forms/tests/it/protocol_control_forms.rs, qsl-semantics/src/check/protocol_controls.rs (tests mod), qsl-semantics/src/check/protocol_memory/tests.rs"
review_set: subset
---

## Summary

Ticket: QSL-1000. The PR adds tests and `#[trace]` tags only (no production
code). Each new or changed tag was checked against the AC text on main for
whether the test actually asserts what the AC states, and whether the gaps the
PR reports as uncovered are real and tracked by QSL-1027 / QSL-1028.

Most bindings are correct: FR-205-AC-6, FR-206-AC-9, FR-220-AC-1/2/7/8/9,
FR-223-AC-7, FR-225-AC-2, FR-309-AC-2/3 assert their ACs. The reported
uncovered parts (FR-223-AC-6/8, FR-223-AC-10 second sentence, FR-219-AC-5
second sentence) are real on main (`sc_event` supersedes a thread's earlier
fence unconditionally, qsl-eval/src/simulation/protocol/ra.rs:675-703) and are
tracked by QSL-1027 / QSL-1028.

## Verdict

**CHANGES REQUESTED.** One false tag (FR-309-AC-1 on a test asserting the span
the AC excludes), one AC whose stated mechanism is false on main but is tagged
as covered and not reported (FR-223-AC-9), one tautological identity check
(FR-225-AC-1), and an FR-219-AC-5 tag on a test that never compares obligation
identities. Lower findings are partial assertions. Tests on a throwaway merge
with origin/main 566ce2bcf: see Scope and process.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-309-AC-1 tag on a test that asserts the span `memory tso`, which the AC (span of `tso`) excludes; the gap is tracked only as a trailing sentence in QSL-1028, a qsl-package ticket | qsl-forms/tests/it/protocol_control_forms.rs:399-420 |
| FND-002 | medium | FR-223-AC-9 tagged as covered, but its clause "the first fence is still in P when v stores P := 1" is false on main (unconditional fence supersede); the test asserts the outcome only, the PR does not report it and QSL-1027 does not list AC-9 | qsl-analyze/tests/it/release_acquire.rs:1066-1077 |
| FND-003 | medium | FR-225-AC-1 obligation-identity half is tautological: the varied max_store_buffer is set on a request that is never passed on; SubjectContext::new reads only request.subject.model, so the three calls are identical | qsl-analyze/tests/it/weak_memory.rs:289-313 |
| FND-004 | medium | FR-219-AC-5 tagged on a test that compares resolved memory values, never obligation identities nor package identities; the AC's second sentence is false on main, so the tag records as covered an AC that is partly unasserted and partly violated | qsl-analyze/tests/it/weak_memory.rs:431-461 |
| FND-005 | low | FR-223-AC-10 tagged on a first-sentence-only test; reported and tracked by QSL-1027, but the matrix shows the AC as covered | qsl-analyze/tests/it/release_acquire.rs:1080-1099 |
| FND-006 | low | FR-218-AC-27 qualified-name asserts are tautological: the test builds "Pipeline::Pay"/"Billing::Pay" with format! from its own lookup keys, then asserts they differ | qsl-semantics/src/check/protocol_controls.rs:571-578 |
| FND-007 | low | FR-220-AC-3 and FR-220-AC-10 say "naming the attempt"; both tests skip the cause's `attempt` field with `..` | qsl-semantics/src/check/protocol_memory/tests.rs:707-713,750-756 |
| FND-008 | low | FR-206-AC-10: the test asserts only that some reached state is registered; it does not check that event(E) is enabled at the state after join(P), or that the root env holds A's binder | qsl-analyze/tests/it/protocol_compensation.rs:1012-1030 |
| FND-009 | low | FR-217-AC-1: the AC's last sentence (envelope steps serialize with transition identities and the blocked list) is not asserted | qsl-analyze/tests/it/protocol_system.rs:994-1022 |

## Scope and process

Methods: code-review with its rust-review lane (tests-only Rust diff). Skipped:
quoin gap-analysis, because the diff has no production code (the two
qsl-semantics files that changed are `#[cfg(test)]` test modules). Spec-review
was not run because nothing under spec/ or plan/ changed. AC texts were read
from spec/functional/FR-205, -206, -217, -218, -219, -220, -223, -225 and -309
on main.

Ran `cargo test -p qsl-forms -p qsl-analyze -p qsl-semantics` on a throwaway merge (PR head merged with
origin/main 566ce2bcf), with its own target directory. Results are in the
Linear comment. The qsl-semantics `complete_value_lock` failures fail only
because the quire-specification checkout is missing (known environment
noise).

## Dispositions

Round 1, verified against fix commit 45d622811ad5e22e3e3dcf0d8535de7941e5022c.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-309-AC-1 tag dropped (`#[trace("TC-888")]`); the doc comment now states the whole-clause span; the gap is in QSL-1028's body |
| FND-002 | fixed | FR-223-AC-9 tag dropped; QSL-1027 does not yet list AC-9's mechanism clause (tracking follow-up, non-blocking) |
| FND-003 | fixed | Obligations are authored per bound and compared by value, with a different-package negative control; the comment states the bound is not an input to the obligation |
| FND-004 | fixed | FR-219-AC-5 tag dropped; the AC is now honestly uncovered (second sentence: QSL-1028) |
| FND-005 | fixed | FR-223-AC-10 tag dropped; the doc comment names the first sentence only |
| FND-006 | fixed | Asserts (protocol, channel) names read from graph.protocols() |
| FND-007 | fixed | `attempt` asserted: Wa/Wb (AC-3) and Lone (AC-10) |
| FND-008 | fixed | At every state after a join step: Event enabled and C's binder held by the root |
| FND-009 | fixed | Prefix transition_key JSON asserted: type protocol-transition, steps fork Main/Both and attempt Main/Both/left/A |
