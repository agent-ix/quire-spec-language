---
id: SR-1091
title: "QSL-394 EARS review of FR-350 to FR-354 and FR-003"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@46c958aefd7f25e22a52768e8289c946e1f74e0b; spec/functional/FR-003, FR-350..FR-354"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: reviews
---

## Summary

Ticket: QSL-394. PR agent-ix/quire-spec-language#581 at 46c958ae. Every
Description is an event-driven EARS statement (When ..., the runner/QSL
SHALL ...), and the Behavior bullets use "If ..., then ... SHALL". Two
statements fall outside the pattern.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-353's Description is compound: one SHALL gives the verdict, a second gives the exit status. The Behavior section already states both separately. | spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md:19-21 |
| FND-002 | low | FR-350's vector isolation ("Each vector runs on its own. A tool-failure ... changes no other vector's outcome") is stated as fact with no SHALL. FR-350-AC-4 tests it, so it is normative. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:78-80 |

## Verdict

The statements conform to EARS apart from two low wording findings. The
semantic and consistency findings are in SR-1090.
