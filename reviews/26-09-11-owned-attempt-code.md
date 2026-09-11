---
id: SR-365
title: "Code and Rust review of own-attempt Boolean choices"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/native/families.rs; src/protocol_artifact/native/families/decisions.rs; src/protocol_artifact/native/families/decisions/received.rs; tests/native_choice_emission.rs; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Actual Claude Opus code/Rust review and focused recheck using repository
conventions and the actual `code-review` and `rust-review` skills. Session
`249b16e1-c003-441b-a4a8-75bfae8a4dec`; retained outputs:
`/tmp/quire-owned-attempt-opus-review.jsonl` and
`/tmp/quire-owned-attempt-opus-recheck.jsonl`.
No ownership, causal-flow, operand-loss or accounting defect was found in the
exact-role Attempt eligibility arm. The existing model, field, formula and
operation-contract authorities remain unchanged.

## Verdict

**PASS.** Opus cleared the substantive findings conditional on the focused
fix gates. The parent verified terminal exit zero, sixteen choice tests,
fifteen protocol tests and both strict Clippy configurations; the condition is
satisfied. No further review campaign is required for this increment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: exhaustive Send/Effect/Event refusal arm and three real typed/discharged negative scenarios establish exact FamilyProof outcomes. | received.rs:152; tests/native_choice_emission.rs:25 |
| FND-002 | medium | Resolved: prior full/supplemental gates and final focused fix gates are terminal pass. No deny.toml exists, so no cargo-deny lane was invented. | /tmp/quire-owned-attempt-gates.log; /tmp/quire-owned-attempt-supplemental.log; /tmp/quire-owned-attempt-review-fixes.log |
| FND-003 | low | Closed as scoped: absence-of-Effect assertions describe output boundaries, not mutation coverage of the new eligibility arm. The explicit effect-record refusal tests that admission boundary. | tests/native_choice_emission.rs |
| FND-004 | low | Resolved: sibling availability test explicitly names inherited linker/type-flow refusal, not new-arm coverage. | tests/native_choice_emission.rs:515 |
| FND-005 | low | Resolved: observed-Boolean terminology, module ownership note and document wrapping aligned without unnecessary renames. | families.rs:3; received.rs:3; FR-042; compiled-protocol-v1.md |
| FND-006 | low | Optional follow-up: the send refusal uses Sender ownership; a Receiver-owned send row would additionally discriminate accidental reuse of channel-to-role eligibility. No current defect or merge blocker. | tests/native_choice_emission.rs:30 |

## Coverage

Real source/model/type/proof/admit/emit/read tests cover cross-unit operation
contracts and original observation owners, foreign-role same-model refusal,
joined distinct atoms and nonpartition refusal, inherited sibling availability,
numeric comparison refusal, References exhaustion/locus/exact/one-short/retry,
and excluded send/event/effect observations. Attempt observations establish no
operation-success or business-effect evidence.

Full pre-fix suites passed 586 minimal and 602 all-feature tests plus five
doctests each; four inherited assurance ignores remain per configuration.
Formatting, both strict Clippy configurations, minimal bins/examples,
warnings-denied rustdoc, both audits and the real stripped-release producer
test passed. The final fix run passed sixteen choice and fifteen protocol tests
plus formatting and both strict Clippy configurations. Full suites were not
repeated for the exhaustive equivalent refusal arm, docs and added negative test.
The reviewer inspected logs; execution belongs to the parent.
QUOIN SR366/367/368 retain their scoped outcomes and inherited corpus debt.
