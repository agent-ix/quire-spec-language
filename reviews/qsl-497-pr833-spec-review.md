---
id: SR-9093
title: "Spec review of quire-spec-language PR #833: FR-086, FR-300, TC-234, TC-241, TC-791 and ADR-016 G-2b"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b23386bb470bd029d311ccf7ed7f50b857589ea6; PR #833 spec edits: spec/functional/FR-086, spec/functional/FR-300, spec/test-cases/TC-234, TC-241, TC-791, spec/decisions/ADR-016 (G-2b row), spec/tests.md"
review_set: subset
---
# Spec review of quire-spec-language PR #833

## Summary

Ticket: QSL-497. Base spec-review only. QSpec FR-152 is cited by id only.

- FR-086 and TC-234: the edit only removes the known-gap notes that the code and tests now close. Nothing normative changes.
- TC-241: the rewrite replaces a "record each record's classification order" hook that cannot exist with an observation the code can make. The Port's kind depends on the interface phase having run, and there are 3 charges. FR-086-AC-5 and QSpec FR-152 name one `systems.kind` charge point for every record, so charge order cannot be seen as a sequence of charges. A failed admission (`Incomplete`) drops all partial results, so the only effect of the order a test can see is the dependency. The rewrite keeps everything about the row that can be tested.
- FR-300-AC-3, TC-791 and ADR-016 G-2b: these now agree. S1 and S3 are reported, S2 is absent by design, and a `state_model::` arm in the declaration dispatch fails the probe. FP-3 says model forms are always nested in `Value` or `ProtocolClause` bodies, which supports the edit. Model declarations come in through domain-package intake, not through a QSL leading token.

## Verdict

PASS. There are no blocking findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Recorded

- medium, recorded: TC-241 step 1 sets the key order to Part, Interface, Port. Under that order a single key-order loop still classifies the Port, which contradicts the row's own mutant claim. The shipped test uses Part, Port, Interface and does catch it. Step 1 should say the Port sorts before the Interface.
- low, recorded: FR-300 and the probe message cite FP-1 for "no leading token", but FP-1 says nothing about leading tokens. FP-3 alone supports the edit.
