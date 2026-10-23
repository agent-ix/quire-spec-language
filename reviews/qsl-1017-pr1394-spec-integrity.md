---
id: SR-16325
title: "Spec integrity review of PR #1394 FR-447 fairness emission clause"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; QSL-1017 PR #1394; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:548-561; QSpec FR-481 hybrid claims:186-192"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
---
# Spec integrity review of PR #1394

## Summary

Ticket: QSL-1017. Reviewed the PR's FR-447 S4 fairness emission edit against its cited QSpec FR-481 wire contract. The two specifications prescribe different members for one source form.

## Verdict

**FAIL.** FR-447 calls the new encoding a QSpec FR-481 member while contradicting FR-481 for `fair S` whenever all transitions of S share a target. This needs an explicit cross-repository contract decision and corresponding spec change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-447 says `fair S` with one transition target emits `declaration` the mode node and `name` the target mode. QSpec FR-481 says every `fair S` emits `declaration` the model node and `name` S, reserving mode-node/target for `fair S::T`. Align the normative contracts before asserting that S4 emits QSpec FR-481 hybrid fairness members. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:555-560; QSpec spec/objects/interfaces/FR-481-checked-package-hybrid-model-nodes.md:186-192 |

## Coverage

Examined the edited FR-447 S4 emission statement, FR-447-AC-12 and AC-16 as context, and QSpec FR-481's hybrid fairness paragraph and AC-9 as context. The rest of the edited statement preserves the FR-481 node forms and the listed profile interval forms. The relevant scope is a targeted integrity review; other optional spec-analysis lenses were not run.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | PR #1394 at `03ed8c629` still contains FR-447 lines 555-560, saying single-target `fair S` emits the mode-node/target member. Open spec PR #1397 adds a correct AC-5/TC-1187 vector but does not yet remove or correct this paragraph. Align the normative paragraph before #1394 merges. |
| FND-001 | fixed | At `fb4fbd25`, the merged FR-447 S4 paragraph requires model-node/mode for every `fair S` and mode-node/target for `fair S::T`, matching QSpec FR-481. The FR-447-AC-5 and TC-1187 `MovingFair` vector also checks the model-node/mode member. This row supersedes the prior still-open disposition. |

## Final disposition scope

Checked the merge's FR-447 S4 and `MovingFair` AC/test-case changes against QSpec FR-481, plus the resulting PR-specific diff. The previously fixed code and gap findings remain outside this narrow spec disposition; no new regression was found in the reviewed merge scope.
