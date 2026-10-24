---
id: SR-16130
title: "Spec review — ADR-011 X-7/X-8 exactly-lists name serde and ix-trace-rs"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md at 0229f4247"
review_set: subset
evaluated_revision: "0229f424771b94dbf867c6a4a33c2abca7cfbae3"
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---

## Summary

Ticket: STD-314. PR quire-spec-language#1275 adds `serde` and `ix-trace-rs` to
ADR-011 §7.3 X-7's `qsl-package` exactly-list (with the purpose of `quire-canonical`) and
`ix-trace-rs` to X-8's `qsl-eval` list. Reviewed `git diff origin/main...HEAD` at 0229f4247
against `qsl-package/Cargo.toml`, `qsl-eval/Cargo.toml` and `tests/it/family_outcome_layering.rs`
on main, ADR-011 X-13, and the heads of #1233 (IR-589) and #1269 (IR-583).

Verified: with `quire-checked` set aside, the X-7 list equals qsl-package's `[dependencies]`
on main (`ix-trace-rs`, `qsl-foundation`, `qsl-semantics`, `quire-exact`,
`quire-contract-model`, `quire-canonical`, `quire-semantic-value`, `serde`, `thiserror`) and
the X-8 list equals qsl-eval's. On the textual-conflict question: neither #1233 nor #1269
edits ADR-011 at all, and `git merge-tree` of #1275 with each, and with every other open QSL
PR, is clean. The risk is semantic, not textual, and it sits with those PRs: #1233 makes
`quire-contract-model`, `quire-canonical`, `quire-exact` and `thiserror` optional and
deletes `serde` from qsl-package, and #1269 adds `quire-eval` to qsl-eval, so whichever lands
after #1275 must edit the X-7 or X-8 row in the same PR or the row goes stale. The PR body
already names both. No new id. `make check-spec-validation` exits 0. Wording is
behaviour-only.

## Verdict

MERGEABLE: one low finding (judgement call, the PR body already flags it as a
lead default). Lead note for sequencing: route an ADR-011 X-7 edit into #1233 and an X-8
edit into #1269 before they merge; that is their scope, not this PR's.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The X-7 and X-8 rows say their `[dependencies]` "are exactly" a list that names `quire-checked` (C, from X-13), but neither manifest nor TC-390's sets name it on main, so after this PR the exactly-lists still differ from the code by that one crate. X-13 says the lists gain it when X-13 lands; the row would read accurately with "from X-13" turned into a condition (for example "and, once X-13 lands, `quire-checked`"), or the lead default stands as flagged. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1225,1226 |

## Dispositions

Round 1, checked against fix commit cb65d65a2. The X-7 and X-8 exactly-lists now equal `qsl-package/Cargo.toml` and `qsl-eval/Cargo.toml` on main, with `quire-checked` stated as X-13's addition. `make check-spec-validation` exits 0; main is an ancestor; no textual conflict with any current open PR. The sequencing note stands: #1233 (drops `serde`, makes four deps optional) and #1269 (adds `quire-eval`) must edit these rows when they land.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `quire-checked` is out of both exactly-lists; each row says "X-13 adds `quire-checked` ... to this list". |
