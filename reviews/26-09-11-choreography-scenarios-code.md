---
id: SR-379
title: "Code review of observed repeats and choreography scenarios"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/native/families.rs; src/protocol_artifact/native/families/decisions.rs; src/protocol_artifact/native/families/decisions/received.rs; tests/native_choice_emission.rs; tests/native_payment_retry_emission.rs; tests/native_split_shipment_emission.rs; tests/native_refund_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Code and Rust review examined the combined scenario branch at 700f061 against
bb30eac. Observed repeat guards reuse the existing owner, atom, visibility and
bounded valuation checks. Split-shipment and refund tests exercise the real
compiler and reader without adding production behavior. No new dependency,
public API, wire format, unsafe code or hosted workflow change is introduced.

## Verdict

**PASS** for the reviewed implementation increment: the reported boundary-test
gaps are resolved and local checks pass.
Campaign acceptance, the specification-cycle decision under issue #66, and the
actual B consumer handoff are separate unfinished work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved implementation findings in the reviewed increment. | - |

## Review evidence

The reported boundary-test gaps were resolved: the nested regression now covers
zero maximum with an observable exhausted child under feasible and infeasible
false valuations. An infeasible true valuation admits a standalone non-progressing
body but cannot supply enclosing progress. The all-feature full suite and the
minimal 23-test choice target both pass with these added controls.

The repeat's body obligation and its contribution to an enclosing loop are
distinct. A feasible false valuation prevents a progress contribution even when
the body is observable. An infeasible false valuation retains the closed-true
behavior. The closed path keeps visible-constant, maximum and guard processing
in its original order. Every continuing body still refuses absent progress or
unproved family authority. The nested regression checks refusal and the positive
counterpart, including retention of the original disjunction in emitted values.

The retry budget test now observes the repeat guard itself and exercises zero,
one-short and exact reference, entry and byte-work ceilings. The zero-maximum
test no longer claims budget coverage. Authored forward attempts retain one
static record; runtime occurrence ordinals are not asserted as wire fields.
These changes address SR-378's earlier attempt-identity defect and the later
repeat-progress and budget findings; SR-378 describes its older baseline.

The three scenario modules assert output identities, subjects, binders, anchors,
causal edges and dependency relations. Adverse controls inspect typed scope or
family refusals. Refunds use a multi-unit fixture and check wrong-kind references
against their owning unit, avoiding the source-index assumption documented in
issue #68. Compiler/reader equality checks establish agreement, while the
scenario-specific assertions independently select the expected records.

Native dependency-seal validation, unchanged wire records and the existing
invocation-local work limits remain the boundaries. No new unowned production
behavior, placeholder, panic path, unchecked conversion or async/locking path
was found in the changed production code. No applicable AssuranceProfile was
found in this specification scope.

## Validation

- Formatting and git whitespace checks: passed.
- Minimal full suite at 700f061: 625 passed, 0 failed, 4 inherited ignored.
- Both Clippy configurations with warnings denied: passed, including rechecks after the added boundary controls.
- All-feature full suite including the added boundary controls: 641 passed, 0 failed, 4 inherited ignored.
- Minimal choice target recheck with the added boundary controls: 23 passed, 0 failed. The production source is unchanged from the minimal full-suite run.
- Pinned scoped specification validation: passed, 435 documents grammar-clean.
- No deny.toml is present, so cargo-deny is not a configured repository gate.

Pinned trace reconciliation uses CLI ff638b98, engine d3bc2baf, process e6ea515
and ISO a60ee12, with both module roots explicit. It reports 374/383 backed
targets, FR-042 9/10, and model-linking test cases 41/42; status classification
ran with no status-column diagnostics or status lies. Those counts measure
binding, not complete behavioral acceptance. FR-042-AC-10 remains outstanding.

The gap-analysis workflow's plan-completion step cannot run because no plan
bundle covers this stage. Issue #66 explicitly uses epics/tickets for these
stages and prohibits creating replacement bundles. No plan-completion PASS or
full-campaign acceptance is claimed. Remaining work: #39, #40, #66.
