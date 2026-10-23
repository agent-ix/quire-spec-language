---
id: SR-16313
title: "spec-review of FR-049 StaticAuthority identities keep the sha256 domain (quire-spec-language PR 1322)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0623a52452; spec/functional/FR-049-admit-composed-evaluation-inputs.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
---

# Spec review of FR-049 StaticAuthority identities keep the sha256 domain (quire-spec-language PR 1322)

## Summary

Ticket: STD-353. One spec-review pass over `git diff origin/main...0623a52452` (agent-ix/quire-spec-language#1322), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- The edit is one sentence in FR-049 Inputs; the rest of the requirement, its ACs and the `valid_digest` description are unchanged.
- No QSpec PR names are cited.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | scope-deferral: main says moving `StaticAuthority`'s four identities to FR-056's `sha256-jcs` digests is tracked work (refs #131). The PR replaces that with a permanent statement that they do not take those digests, a planned capability dropped by lead default. The standing rule makes a dropped or permanently refused capability a blocking finding even when the PR body flags it as "lead default, not an owner answer". Fix: spec the move (the digest domain each of the four identities takes and the refusal for a wrong-domain digest), or obtain Peter's ruling that the move is dropped and record it as his. | spec/functional/FR-049-admit-composed-evaluation-inputs.md | missing-requirement |
| FND-002 | medium | The justification cannot be checked in the spec set and the new statement has no AC. ADR-011 never names `StaticAuthority` (the only artifact that does is FR-049); its M-6c row deletes the `state` and `temporal` evaluators and its M-6e row the composed checker, so "part of the native state evaluator, which ADR-011 M-6c deletes" is not a stated fact. The PR also keeps a `sha256`/`filament-canonical-json-1` domain requirement with no vector, on the argument that a test would pin code QSL-834 deletes. Fix: cite the ADR row that deletes `StaticAuthority`, or state the domain requirement as an AC. | spec/functional/FR-049-admit-composed-evaluation-inputs.md | missing-requirement |

## Verdict

Not mergeable: FND-001 (high) block the PR; FND-002 (medium) should be fixed in the same round.

Merge-tree clean against origin/main and every open QSL head.
