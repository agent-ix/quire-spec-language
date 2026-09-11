---
id: SR-366
title: "Gap analysis of own-attempt Boolean choice admission"
type: SpecReview
analysis: gap-analysis
scope: "src/protocol_artifact/native/families/decisions.rs; src/protocol_artifact/native/families/decisions/received.rs; tests/native_choice_emission.rs; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` reviewed `agent-a/received-choice-handoff...ff5e22a`.
The production change adds only a narrow Attempt eligibility arm: after the
existing EventRecord, control-anchor and flow checks, it resolves the attempt
role at that control and requires exact SymbolId equality with the choice owner.
Six focused tests cover identity/reader round trip, foreign-role refusal, joined
distinctness and invalid partition, sibling scope refusal, non-Boolean refusal,
and References exhaustion/exact/one-short/retry behavior.

## Verdict

**FAIL** — only under the inherited strict corpus matrix rule. Four ordinary
unbacked rows are outside this scope; no scoped ownership, resource, or reverse
code-to-requirement gap was found. Full gates were non-terminal at initial
review; SR-365 records their final passing results and separate Opus recheck.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Four inherited source-backed-required rows retain the strict matrix FAIL but are unaffected by own-attempt admission. | TC-115; FR-036-AC-5; FR-036-AC-6; FR-036-AC-8 |
| FND-002 | low | Full AC-9 vectors across every artifact dimension remain broader corpus debt; this increment supplies own-attempt References boundary evidence only. | FR-042-AC-9; TC-121:174-176; native_choice_emission.rs:57-164 |
| FND-003 | low | No scoped reverse gap or stub found: only Attempt is added; Effect, generic Event and non-EventRecord observations remain excluded. | received.rs:111-152; FR-042:95-109 |

## Coverage

Reconciliation: `quire coverage --scope /home/peter/dev/worktrees/quire-language-owned-attempt-choices --json` (quire 0.31.0, engine `ca7362d4`; `/tmp/quire-owned-attempt-coverage.json`) reports 374/383 rows backed. TC-010 (Manual) and FR-017-AC-2 (Inspection) are `no_source_symbol` exemptions; FND-001 lists the four ordinary gaps. `status_lies` is empty, but inherited `status-column-matches-nothing` diagnostics qualify that result. FR-042 is 9/10 backed corpus-wide; the six new tests carry TC-121 and relevant FR-042 tags.

No plan bundle covers protocol-artifact emission (SR-355 precedent), so plan completion is inapplicable and no plan was fabricated. Optional semantic review was not selected. Parent reports the focused 15-test lane and both strict Clippy configurations passed; full gates at `/tmp/quire-owned-attempt-gates.log` were not terminal at review time.
