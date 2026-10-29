---
id: SR-1830001
title: "code review of PR #1830 (QSL-518, partial): FR-345 generated layering corpus and layering-check"
type: SpecReview
analysis: code-review
date: 2026-10-29
scope: "agent-ix/quire-spec-language; git diff origin/terra-qsl-integration...ebd5adea8 (xtask refinement layering_corpus.rs, layering_check.rs, layering.rs, refinement.rs, error.rs; Makefile; FR-345, TC-868)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1830001: code review of PR #1830 (QSL-518, partial)

## Summary

Head ebd5adea8. The review covers code review and Rust review.

**Nothing is committed or pinned.**
- The diff adds no fixture, corpus or model file.
- `REAL_CORPUS` is removed. It pointed at `tests/fixtures/refinement/layering/`, which never existed at the base.
- `generate` computes each model digest from the bytes at run time, through `package_input`.
- The zero `PLACEHOLDER` values are manifest and lock fields of the generated timed model. They are not the identity of anything the gate checks.
- `make refinement-layering` writes into a `mktemp -d` directory and removes it on `EXIT`.

**`layering-check` fails closed.**
- It reads the report into typed structs, without `deny_unknown_fields` on the outer report.
- An unknown result `kind` fails as `NotAReport`.
- It requires exit 21 and verdict `unsupported`/21.
- It requires every case and edge to hold or be not applicable.
- It requires exactly one result per known layer, six in all, each with one admitted, not-negotiated witness over a non-empty candidate set where every candidate is `not-negotiated`.
- Over the real report its structs parse: 12 cases, 6 edges and 6 layers.

**The witness item.** It is now the unit's single `StateClauseKind::Invariant` clause. The temporal witness's `pre` and `post` clauses no longer count against "exactly one".

**The real run** at this head (corpus generated into the scratchpad):
- The CLI exits 10.
- Five layers are regressions over an empty candidate set, because the production registry has no candidates (QSL-1200/1201).
- The temporal witness is `unsupported_construct`/`not-yet-implemented` (FR-230 timed emission).
- `layering-check` rejects this run: "the CLI exited Some(10), not 21".

This is the state the PR body declares. AC-9, AC-10, AC-12 and the temporal witness half remain, so `refinement-layering` correctly stays out of `CI_CHECKS`.

**Gates at ebd5adea8**, all exit 0:
- `cargo clippy -p xtask --all-features --all-targets -D warnings`;
- `xtask --test refinement_layering`: 6 passed;
- `xtask --lib layering`: 20 passed;
- `quire validate` on FR-345 and TC-868.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | FR-345 |

## Verdict

Mergeable.
