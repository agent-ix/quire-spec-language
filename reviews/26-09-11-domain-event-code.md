---
id: SR-374
title: "Code and Rust review — same-owner domain-event Boolean choices"
type: SpecReview
analysis: code-review
scope: "FR-042; TC-121; src/protocol_artifact/native/families/decisions.rs; src/protocol_artifact/native/families/decisions/received.rs; tests/native_choice_emission.rs; tests/native_domain_event_choices.rs; tests/native_domain_event_boundaries.rs; tests/support/native_protocol/mod.rs"
review_set: subset
---

## Summary

Actual Claude CLI Opus reviewed the domain-event change against main `3b1dae7`,
including both initially untracked test files, then rechecked the shared-helper
lint fix and the added commit-record refusal test. The reviewed implementation
is checkpointed at `455b821`; no blocking code finding remains after the targeted
rechecks. The added commit-record test subsequently passed the parent gate.

## Verdict

**CONDITIONAL** — substantive findings are resolved as authored and rechecked;
the remaining low findings are optional cleanup. Execution evidence below is
from terminal parent checks, not inferred from this static review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The three choice test binaries repeat the proof-discharge assertion helper. Optional shared-helper consolidation would reduce maintenance drift; it is not a behavior defect or merge blocker. | tests/native_choice_emission.rs:765; tests/native_domain_event_choices.rs:108; tests/native_domain_event_boundaries.rs:356 |
| FND-002 | low | The two new binaries duplicate the same-model foreign-role refusal scenario. Either copy can be removed in a cleanup without reducing that scenario's coverage. | tests/native_domain_event_choices.rs:381; tests/native_domain_event_boundaries.rs:267 |
| FND-003 | low | Private `Received` naming now covers receives, attempts and domain events. Updated module documentation states the broader scope correctly; renaming is optional clarity work. | src/protocol_artifact/native/families/decisions/received.rs:29 |

## Resolved findings

The first review found that the event-only boundary binary did not use shared
`Inputs::step_contracts`, causing the actual strict lint gate to fail. The
method-local `#[allow(dead_code)]` and its shared-fixture explanation at
`tests/support/native_protocol/mod.rs:235` resolve that finding as authored.
The allowance does not weaken production or crate-wide lint policy. Opus
confirmed that `expect(dead_code)` would instead fail in binaries which do use
the method, so the localized allowance is appropriate to this shared test seam.

The first review also found no test for the newly stated commit-record
ineligibility boundary. The added
`a_commit_record_does_not_supply_a_boolean_decision_fact` at
`tests/native_domain_event_choices.rs:694` places a same-role Boolean decision
after the actual commit. Its helper first requires typed/discharged source,
then asserts `Unsupported::FamilyProof`: an out-of-scope input cannot satisfy
the prerequisite, and complementary Boolean guards avoid an unrelated partition
failure. The same-session targeted recheck accepted this as a substantive
regression assertion. Its subsequent focused execution passed alongside the four
existing domain-identity tests, followed by both strict Clippy configurations.

Overbroad AC-8 trace tags were removed from the new tests; the narrower trace
claims were accepted during the helper-fix recheck. No claim is made that these
tests qualify every budget dimension or complete all FR-042 acceptance.

## Review scope and authority

The review used the actual `code-review`, `rust-review`, `rust-style` and
implementation-gap discovery instructions, with repository conventions taking
precedence. No applicable AssuranceProfile was found in the specification tree.
No workflows changed, no new engine was introduced, and no broad assurance
campaign was substituted for the requested PR review.

The new match arm reuses structural role references at the exact event site
and compares resolved role identity, not model type or display name. Existing
causal availability, visibility, selected Boolean field, binder and observation
anchor authority remain prerequisites. Send/effect/commit exclusions remain
separate from event eligibility. Existing metered decision work remains the
resource authority, exercised by exact/one-short and original-locus tests.

Compensation-qualified events retain the existing explicit registration
association and runtime bindings. The baseline-versus-choice assertions retain
registration, activation, attempt and effect binding identities; admitting the
Boolean field establishes none of those runtime occurrences or predicate truth.

## Execution evidence

The initial parent gate log records 18 existing choice tests, four domain-event
identity tests, four boundary tests and 15 native protocol tests passing, followed
by the actual shared-helper lint failure. This is evidence for that earlier
checkpoint, not a pass for the latest commit-refusal addition.

The full gate completed successfully: 597 minimal-feature tests and 614
all-feature tests, plus five doctests per configuration and four inherited
ignored tests per configuration. The added commit refusal was present in the
all-feature run; the later focused minimal run passed all five domain-identity
tests. Both strict Clippy configurations, formatting, bins/examples build,
warning-free rustdoc and fixture-audit self-test/model-byte checks passed.
Logs: `/tmp/quire-domain-event-full-gates.log` and
`/tmp/quire-domain-event-supplemental.log`. PR-63's separately validated mixed
tests were integrated after the full-suite target inventory was selected;
the subsequent all-targets Clippy checks include them. The separately named
stripped-release producer check is recorded at PR handoff, not inferred here.

## Review provenance

Actual read-only Opus session: `b89c32ef-8447-47a7-92a4-6b97c5ad0d59`.
The initial review and both narrow continuations completed successfully. Local
transcripts are `/tmp/quire-domain-event-review-opus.jsonl`,
`/tmp/quire-domain-event-review-recheck.jsonl`, and
`/tmp/quire-domain-event-review-commit-recheck.jsonl`. These identify the actual
review process; they are not runtime identity catalogs or substitutes for tests.
