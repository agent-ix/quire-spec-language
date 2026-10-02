---
id: SR-358
title: "Code and Rust review of native received-Boolean choices"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/native/families.rs; src/protocol_artifact/native/families/decisions.rs; src/protocol_artifact/native/families/decisions/formula.rs; src/protocol_artifact/native/families/decisions/received.rs; tests/native_choice_emission.rs; tests/native_protocol_emission.rs; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Actual Claude Opus code/Rust review and focused finding recheck, using the actual
`code-review` and `rust-review` skills and repository conventions. Session
`df9d5a76-871b-4ee8-9329-c8f12ad19465`; retained outputs:
`/tmp/quire-boolean-opus-review.jsonl` and
`/tmp/quire-boolean-opus-recheck.jsonl`.

The reviewer found no soundness defect in receiver ownership, original
atom/provenance identity, visibility, bounded independent-valuation partitions,
closed/dynamic classification or repeat progress. All three substantive initial
findings are cleared by source/tests/contract changes.

## Verdict

**APPROVE.** The parent verified the focused fix gates passed. No blocking
implementation finding remains. The parent owns execution verification;
the read-only reviewer did not run commands.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: real admitted Object Boolean-field source now runs through emission/read with exact model/export identity. Missing object record is rejected by NativeModel admission and cannot reach this defensive Invalid::Model branch through public constructors. | tests/native_choice_emission.rs:24; src/native_model/admission.rs:477; received.rs:246 |
| FND-002 | medium | Resolved as contracted: private per-choice read/lowering records charge cumulative Entries, including transient records. Two dynamic choices have measured default/exact/one-short Entries and fresh retry/reader evidence. No caching redesign required. | docs/compiled-protocol-v1.md:614; FR-042; tests/native_choice_emission.rs:79 |
| FND-003 | medium | Resolved: an explicit feasibility/case length check returns Invalid::Control before zip, preventing a future invariant break from skipping progress checks. | src/protocol_artifact/native/families.rs:210 |
| FND-004 | low | Nonblocking: borrowed immutable model/record/field pointer identity and redundant anchor comparison remain conservative. | received.rs:205 |
| FND-005 | low | Nonblocking: expression work reports its actual expression locus; valuation exhaustion is tested at the decision locus. | decisions.rs:51; decisions.rs:103 |
| FND-006 | low | Clarified on recheck: capture wording constrains actual provenance, not atom eligibility. Activation captures precede run and compensation captures are outside choice scope; no valid captured receive was shown to be omitted. Optional wording refinement only. | FR-042; docs/compiled-protocol-v1.md; decisions.rs:171 |
| FND-007 | low | Partially resolved by Entries boundary vectors; complete per-dimension AC-9 evidence remains inherited assurance debt, not claimed complete. | tests/native_choice_emission.rs; FR-042-AC-9 |
| FND-008 | low | Nonblocking: duplicate small FamilyProof constructors and bounded charged linear scans remain; no premature optimization requested. | decisions.rs:32; formula.rs:282; received.rs:205 |

## Coverage

Eleven traced choice tests plus amended protocol vectors cover object/record
atoms, aliases/operators, all-join availability, distinct-atom partitions,
cross-unit contracts, original operands, receiver/visibility refusals, abstract
overlap/hole, feasible-branch progress, and References/Entries resource outcomes.
Internal-consistency guards unreachable through admitted inputs are not
misrepresented as publicly constructible adverse fixtures.

Before the fixes, terminal full local gates passed 580 minimal/596 all-feature
tests plus five doctests each, with four inherited ignored tests each. Both
strict Clippy lanes, formatting, minimal bins/examples, warnings-denied rustdoc,
both audits, the actual stripped producer and 435-document validation passed.
Evidence: `/tmp/quire-boolean-final-gates.log`.
Focused final fix gates passed in `/tmp/quire-boolean-review-fixes.log` (terminal
exit zero): eleven choice tests, fifteen protocol tests, formatting and both
strict Clippy configurations. The full baseline suites were not rerun for this
defensive invariant guard, added tests and documentation-only fix set.
Strict gap FAIL and broader B/related-instance acceptance remain separate.
