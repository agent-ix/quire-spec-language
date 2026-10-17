---
id: SR-11183
title: "code review of quire-spec-language PR 860 (remove hand-maintained TestMatrix files)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e797a8c434bd1087355bce79d47bfca1e2886232; tools/check-index-completeness.sh, Makefile, AGENTS.md, qsl-replay/src/request.rs, qsl-semantics/src/check/identity.rs, qsl-semantics/src/model/intake.rs"
review_set: subset
---

## Summary

Ticket: QSL-784. Code and tooling half of the PR, read in place with no build.

- `tools/check-index-completeness.sh` keeps one `check_kind "FR"` call over
  `spec/spec.md`. `bash -n` passes and the script exits 0 at the head. The
  removed TC half only grepped `| TC-NNN` rows out of the deleted matrices.
  No xtask, CI step or Makefile target parses its output; `ci.yml` and the
  xtask tests reference only the script and target names, which are unchanged.
- No `.rs`, `.sh`, `.toml`, `.yml`, `.json` or Makefile line reads `tests.md`
  or `TestMatrix`.
- The three Rust edits change `///` doc comments only.
- The new AGENTS.md "Trace tag ids" section is a five-line rule, not a ledger.
  It is the convention behind the `QSpec-` prefixed `#[trace]` tags used across
  qsl-cst, qsl-eval and other crates. It moved out of the deleted TM-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. The gate keeps its FR half intact. Only the tracking-only TC half is
removed, and nothing else consumed it. The doc-comment edits are comment-only.
The AGENTS.md section states a real tagging rule.
