---
id: SR-12366
title: "Code review (Rust lane) of QSL-596 part 1: the analyze operation over EN-1 and EN-4 (PR #1362)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d5d3c48f4029cd6ecfd3499bfcede27d4e2607ac; diff 19243dd26..d5d3c48f4 only (19243dd26 = origin/main c21bc57a5 merged with #1299 head c7046ccfb; #1299 files excluded): qsl-analyze/src/analyze.rs, qsl-analyze/src/analyze/items.rs, qsl-analyze/src/model_check/manifest.rs, qsl-analyze/src/lib.rs, qsl-analyze/src/model_check/mod.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/model_check/settle.rs, qsl-replay/src/outcome.rs, qsl-replay/src/lib.rs, qsl-analyze/tests/it/{analyze,closure_certificate,model_settle,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
---
# SR-12366: Code review (Rust lane) of QSL-596 part 1: the analyze operation over EN-1 and EN-4 (PR #1362)

## Summary

Ticket: QSL-596. PR #1362, head d5d3c48f4. The branch is origin/main plus a merge of #1299's older head c7046ccfb (commit 19243dd26, a clean merge). This review covers only `git diff 19243dd26 d5d3c48f4`, the PR's own commit. The #1299 files (`statistical/provider.rs`, `statistical_witness_replay.rs`, the qsl-570-rem review files) are excluded. `/code-review` and `/rust-review` ran as one pass.

Test run: `cargo test -p qsl-analyze --test it -- analyze::` at d5d3c48f4, 9 passed.

## Verdict

**Not mergeable: one high finding (FND-001).**

Checked and correct:

- **Routing.** Routing reads only the descriptors through `Registry::candidates_for_evidence`. No candidate gives `unsupported` with `ClaimKind{kind}`. Exactly one candidate routes the item. More than one is refused and never chosen.
- **One record per item, in request order.** Records come out in input order. The three-item test asserts the indices. Registration and ambiguity are both refused before any item runs.
- **Cancel.** The handle is checked before each item. An already-cancelled handle gives `Incomplete{Cancelled{source}}` and keeps the cause. The handle is also passed into `check_model`/`serve_request`, so a cancel during a run settles through each engine's own map.
- **Determinism.** Routing and settlement are pure functions of the descriptors, the items and the handle.
- **The Proof/Counterexample settlement map.** It matches the FR-281 Behavior list: checker `Ok` gives `Certified`, `Err` gives `CertificateRejected{rule, at}`, no checker gives `Uncertified`; `Reproduced` gives `Refuted`, `NotReproduced` gives `ReplayParity`, `Refused` gives `from_replay_refusal`, which carries the catalog code.
- **`outcome_item`.** An `UndefinedEvaluation` counterexample on a `refuted` record writes `undefined-evaluation`, never the `undefined` label.
- **EN-1 manifest.** It has the same shape as EN-4's. Strong fairness is real in EN-1 (`model_check/refine.rs:248`).
- **API changelog.** The changelog in the PR body matches the public items the diff adds.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `TerminalRecord::settled` is a new `pub` constructor that takes any `TerminalValue`, an optional backend and no evidence. It reopens the forgery that `TerminalRecord::new`/`proved` and their `compile_fail` doctests exist to prevent ("A terminal value alone is not enough evidence to construct a record"; "There is no constructor that accepts a bare `Proved` value", proof_result.rs:1059-1088). Now `TerminalRecord::settled(i, None, TerminalValue::Refuted, None)`, or `Proved{Certified}`, compiles from any crate, and the doctests no longer guard anything. `ModelCheckRecord::into_terminal` (pub, over a struct with all-pub fields) wraps it, and its `Refuted` record carries no replay result. Fix: keep the constructor `pub(crate)`. Move the Proof/Counterexample settlement into qsl-replay as a function that takes typed evidence (a checker acceptance, a `ReplayResult`). Give EN-1 a qsl-replay-owned conversion that keeps the replay evidence. | qsl-replay/src/proof_result.rs:1192-1211; qsl-replay/src/model_check/settle.rs:143-151 |
| FND-002 | medium | `analyze` settles on evidence the engine itself supplies. `Proof.certificate` is a closure that comes inside the engine's report, so the engine picks its own "checker". Likewise `EngineReport::Counterexample`'s closure decides `Reproduced`, and `EngineReport::Model`/`Statistical` accept any pre-built record (the AC-7 test passes a hand-settled record through a `test-model` engine). Because `Engine::new` and `AnalyzeItem::new` are public, any caller can obtain `certified` or `refuted` without FR-282's core checker or layer-6 replay. FR-281 says `analyze` runs the checker and replays the counterexample, and TC-763 puts test engines in layer A "under `cfg(test)`". This is latent: no production engine reports Proof or Counterexample yet. Fix: `analyze` looks up the checker by backend id in a core-owned table and runs replay itself on the engine's payload; make the Model/Statistical variants constructible only by the crate's adapters; put the test-engine seam behind `cfg(test)` or a test feature. | qsl-analyze/src/analyze.rs:62-68,134-158,175-192,362-384 |
| FND-003 | low | `record.clone().into_terminal(index, backend)` clones the whole `ModelCheckRecord` (counterexample, hints) only to read its value. Clone only the value, or have the conversion take `&self`. | qsl-analyze/src/analyze.rs:350 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The public API changelog in the PR body is stale after the fix round. It still lists `EngineReport`, `Proof`, `CounterexampleReplay`, `CertificateRejection`, `TerminalRecord::settled` and `ModelCheckRecord::into_terminal`, which are now removed or crate-private. It omits the new public `AnalyzeItem::unadapted`, `TerminalRecord::unrouted`, `ModelCheckRecord::to_terminal` and `TerminalRecordRefusal::RefutedWithoutCounterexample`. Update the PR body. | PR #1362 body |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | `settled` is deleted and `unrouted` only builds `unsupported`. But `ModelCheckRecord::to_terminal` is public over a struct whose fields are all public, so any crate can write `ModelCheckRecord { value: Refuted, counterexample: Some(<made up>), .. }` and get a `refuted` `TerminalRecord` with no `ReplayResult`. That is weaker than `TerminalRecord::new`, which requires one. Fix: mark `ModelCheckRecord` `#[non_exhaustive]`, so only qsl-replay's settlement builds one (no crate outside qsl-replay builds one by literal; checked by grep), or keep a private field. |
| FND-002 | fixed | c09ac17f0 |
| FND-003 | fixed | c09ac17f0 |

### Round 2 (8c4fdf57e)

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c4fdf57e. `ModelCheckRecord` is `#[non_exhaustive]`, so only qsl-replay's settlement maps build one, and `to_terminal` refuses a `refuted` value with no counterexample. Residual, not this PR's regression: the public fields can still be mutated on a real record, and main's public `TerminalRecord::proved` already mints `certified` with no evidence. |
| FND-004 | fixed | 8c4fdf57e. The PR body changelog now lists the round-1 API (`unadapted`, `unrouted`, `to_terminal`, `RefutedWithoutCounterexample`, `#[non_exhaustive]`), and the removed items are gone. |
