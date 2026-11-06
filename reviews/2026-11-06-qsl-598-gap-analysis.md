---
id: SR-20261105101077
title: "Scoped gap analysis of QSL-598 ToolFailure terminal records"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-598; PR #2464 head fffc763d6; FR-291-AC-1..AC-3, TC-775, FR-290, FR-293; QSpec FR-462, FR-331 (origin/main b09800f7); qsl-replay/src/proof_result/tool_failure.rs, qsl-replay/src/outcome.rs, qsl-replay/tests/it/tool_failure.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-291
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-775
    type: reviews
---

## Summary

Ticket: QSL-598. Planless, diff-scoped gap analysis. Plan completion: not assessed. The acceptance-criteria-to-test mapping is a manual check of the `#[trace]` tags in qsl-replay/tests/it/tool_failure.rs against FR-291 and TC-775; `quire trace` did not return within 120 s.

## Verdict

**ONE SPEC GAP; CODE CORRECT.** Each of FR-291-AC-1, AC-2 and AC-3 has one tagged TC-775 test that exercises production code with real assertions; no stubs or coverage inflation. The variant set is exactly QSpec FR-462's fourteen `defect` rows, spelled identically. FR-290 (certification, plugin readers) and `TimedOut` were already on main; FR-293 is retired with no successor, so nothing is owed there.

The cause discriminator question is settled in favour of the code. QSpec FR-331 spells a backend-envelope cause with `type` (FR-331-AC-40, `{"type": "tool-failure", ...}`), and FR-290's reader tables use `type` for the plugin wire. The QSL outcome document is a different document: every item cause it writes uses `kind` (`ItemCause::kind`, the `replay-parity`, `undefined-evaluation`, `limit-reached` and `not-stochastic` tests), and QSL's own spec states it for an FR-331 cause: FR-357-AC-12, "The outcome document writes a `ScalarAgrees` cause as `{"kind": "scalar-agrees"}` (FR-286)". FR-286 says outcome items carry the label and cause "in FR-331's wire spelling", which governs the label and cause names, not the discriminator member. FR-291-AC-1's `{"type": ...}` copied the envelope spelling into an outcome-document criterion. It is a spec fix, not a code fix.

## Scope

- FR-291-AC-1 (examined): fourteen variants serialize as internal failure with `tool-failure{defect}`, byte-limit payloads, exit 30, read-back.
- FR-291-AC-2 (examined): `TimedOut` and cancelled incomplete, exit 22.
- FR-291-AC-3 (examined): sibling record byte-equal, outcome exit 30.
- TC-775 (examined): procedure and expected results name `tool-failure` and `defect` without a discriminator member; no edit needed.
- FR-286 (context_only): outcome document causes in FR-331 wire spelling.
- FR-357-AC-12 (context_only): outcome document writes an FR-331 cause with `kind`.
- QSpec FR-462 (context_only): failure table, fourteen `defect` values.
- QSpec FR-331-AC-40 (context_only): envelope cause spelled with `type`.
- FR-293 (context_only): retired; nothing to build.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-291-AC-1 states the outcome cause as `{"type": "tool-failure", "defect": D}` and "no member beside `type` and `defect`", contradicting the outcome document's `kind` discriminator (FR-357-AC-12, every `ItemCause`). Edit: replace `{"type": "tool-failure", "defect": D}, FR-331's FailedCause spelling` with `{"kind": "tool-failure", "defect": D}, FR-331's tool-failure{defect} in the outcome document's spelling (FR-286)`, and `beside type and defect` with `beside kind and defect`. TC-775 needs no edit | spec/functional/FR-291-bound-plugin-runs-and-settle-plugin-failures.md:103 |

## Evidence and limits

`cargo test -p qsl-replay --test it tool_failure` (locked-build.sh, shared target, jobs 4, debug 0): 3 passed. QSpec read from a temporary detached worktree of origin/main (b09800f7), removed afterwards. No full gate run.
