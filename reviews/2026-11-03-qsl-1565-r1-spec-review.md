---
id: SR-20261103112803
title: "QSL-1565 r1 spec review of PR #2194 (FR-049-AC-2, TC-136, FR-090 rows)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2194 head 103e438d4; spec/functional/FR-049-admit-composed-evaluation-inputs.md; spec/test-cases/TC-136-admit-composed-state-views.md; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-136
    type: references
---

## Summary

Ticket: QSL-1565. This review covers base spec review and the integrity sub-analysis. The FR-049 and TC-136 diffs against the parent of `d55ffb9d3` (#2173) are empty: the interim first-read amendment is reverted byte for byte. The three FR-090 `object-field-admission-*` rows widen to the call and expression entry, which matches `slot_admission::entry_fault` and `fault_identifier`. The entry refusal shape matches TC-136 H5's `evaluate` precedent: an `invalid_runtime_input` refusal at admission, no `Evaluation`, the meter unchanged.

## Verdict

**MINOR GAPS.** The revert is exact and the FR-090 rows agree with the code. The restored FR-049-AC-2 now claims admission-time refusal on every entry, which the code meets only on the call and expression entries. FR-090's statement of `call`'s failure type does not name the `Admission` variant the PR returns from `call`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Restored FR-049-AC-2 says surplus, foreign and wrong-type inputs "refuse at admission". The public entries that take a raw `ObjectEnvironment` and skip `admit_arguments` (`evaluate_holds`, `evaluate_guard`, `evaluate_hyper_holds`, `evaluate_protocol_slot`) refuse only at the first read of an ill-typed attribute. They never refuse a surplus attribute, an unread ill-typed attribute or an undeclared object type. The PR's own doc comments state this residual, but the spec no longer does, and no ticket owns it. | spec/functional/FR-049-admit-composed-evaluation-inputs.md:149; qsl-eval/src/value/expression/mod.rs:1892-1990 |
| FND-002 | low | FR-090 "Bad arguments are refused at admission" states `CheckedPackage::call` returns `CallFailure { Input(InputRefusal), Fault(InternalFault) }`. The PR returns `CallFailure::Admission(Refused)` from `call` and `call_supplied` for an ill-typed object environment. TC-136 H5 covers that shape for `evaluate` only. The FR-090 text, or an FR-049 or TC-136 row, should name the `call` case. | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:321-329; spec/test-cases/TC-136-admit-composed-state-views.md:58 |

## Dispositions

- FND-001: fixed by stating the scope; no admission was added to the other entries. FR-049-AC-2 now says that `call`, `call_supplied` and `evaluate` admit the object environment whole at entry. It also says that `evaluate_holds`, `evaluate_guard`, `evaluate_hyper_holds`, `evaluate_protocol_slot` and the clause entries evaluate over FR-106-admitted state and refuse an ill-typed attribute at its read with the same code and cause. Those entries are temporal and explorer hot paths (LT-16), so no O(environment) walk was added to them. TC-136 gains row H6 for the entry refusal.
- FND-002: fixed. FR-090's "Bad arguments are refused at admission" now lists `Admission(CallAdmissionFailure)` in `CallFailure` and names the object-environment entry refusal from `call`, `call_supplied` and `evaluate` (FR-049-AC-2, TC-136 H6).
