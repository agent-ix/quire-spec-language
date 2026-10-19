---
id: SR-13341
title: "EARS review of STD-215 hybrid solver budget statements (PR #1056)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@9ba8384c57223d2cbc3f592bd7e5b7b03a1159d1; spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
---
# SR-13341: EARS review of STD-215 (PR #1056)

## Summary

Ticket: STD-215. EARS conformance of the requirement statements the diff
adds or edits in FR-250's Behavior section: the new `hybrid.limits` bullet,
the `max_solver_time` bullet, the `TimedOut` / `Cancelled{source: deadline}`
bullet and the `max_solver_steps` bullet. The repository grammar linter
(`make check-spec-validation`) reports nothing on FR-250.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Two new obligations have no agent: "`hybrid` and its `limits` are never empty" is a bare indicative, and "a fractional wire value is refused before consumption" is agentless passive with no diagnostic. QSpec FR-193 assigns the refusal to the provider ("a provider SHALL refuse ... as FR-331 refuses any out-of-domain value"). Name the refusing component (QSL's request reader or built-in hybrid provider) and the refusal, so AC-3's "is refused" has a checkable oracle. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:86-87,94-95 |

## Verdict

One low finding. Examined and clean for EARS form: the `TimedOut` bullet
(ubiquitous, subject "a solver ... and a provider", one outcome per event),
the frontend-deadline sentence, and the `max_solver_steps` bullet.
