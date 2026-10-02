---
id: SR-1061
title: "EARS review of PR #576 (FR-275 to FR-299 requirement statements)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@546ff375650521c04e3dd0e1a4a7fb35b660ac93; spec/functional/FR-275..FR-299 Description and Behavior statements, FR-027 and FR-100 amendments"
review_set: subset
---

## Summary

Ticket: QSL-390. Checked every SHALL statement and Behavior bullet of FR-275
to FR-299, plus the amended FR-027 and FR-100 paragraphs, for EARS form
(ubiquitous, event-driven "When", unwanted-behaviour "If ... then",
state-driven "While"), a named subject, and observable behaviour. Most
bullets are well formed. Each one names its subject (the operation, the
cache, the plugin host, layer R), and the conditional forms use When or If.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "with a match arm for every category and no catch-all arm" states a Rust coding style, not observable behaviour. "map each O-16 category to the code the table states" already says what can be tested (FR-285-AC-1). Drop the clause. | spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:64-65 |
| FND-002 | low | "Each operation shall run only its own stage" is ambiguous (does `check` running S1 to S2 internally violate it?), and no AC tests it. Say what is observable, for example that `check` takes a `ParsedSource` and does not re-run S1 or S2, or delete the bullet. The type-chain bullet above already covers stage separation. | spec/functional/FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md:76 |
| FND-003 | low | "The smallest path that crosses the CLI, the library, a provider and the cache SHALL work end to end" uses an unmeasurable "work" and slice framing ("smallest path", from ADR-029 SL-1). The Behavior bullets are testable; reword the SHALL to the observable outcome (each command writes its outcome document and exits by FR-285, and a repeated `prove` reads the cache). | spec/functional/FR-299-run-one-source-end-to-end-through-cli-library-provider-and-cache.md:39-40 |

## Verdict

Three low wording findings. None blocks merge on its own.

## Dispositions

Round 1, reviewed at bc11b32cc7c51954c6f8f8f17c68c7949605bf52.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bc11b32c: The Rust match-arm clause is gone from FR-285. |
| FND-002 | fixed | bc11b32c: FR-275 states the observable rule, and AC-5 tests it. |
| FND-003 | fixed | bc11b32c: FR-299 has no 'smallest path ... SHALL work' sentence. |
