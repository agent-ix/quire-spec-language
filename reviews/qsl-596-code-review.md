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

## New findings (disposition pass 3)

Delta 877c0f924..4a190547d (the STD-444 rework; the main merge 877c0f924 is excluded; #1299's `statistical/provider.rs` is ignored). Checked and correct: `analyze` returns one record per item in request order, with no whole-request refusal. `unknown-backend` and `inconsistent-candidates` run no engine. `DuplicateEngine` leaves the set unchanged. The `unbounded-extent` payload is (kind, candidate, advertised modes). `NotStatistical` gives `invalid-request`. `ClaimKind` and `claim_kind` are fully removed. No new public path mints `certified` or `refuted`: `Engine::new` and `EngineReport` stay crate-private, `failed_with` only builds `failed`, and `Disposed` carries no result. The PR-body changelog matches the net API against main.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | `model_refusal` maps every `Liveness(_)` and `Hyper(_)` refusal to `unsupported`, `unsupported_construct`/`not-yet-implemented`. FR-281's table gives `unsupported` only to a refusal whose own FR-272 code is `unsupported_construct` or `unsupported_projection`; any other refusal is `failed`. `LivenessRefusal::UnknownOperation` (a fairness constraint naming an undeclared operation), `HyperRefusal::SubjectCount` (the request supplies the wrong number of subjects), `HyperRefusal::UnknownOperation`, `NoPartnerPath` and both `Automaton(_)` arms are request or engine defects, and they are reported as "not yet implemented". Top-level `Translation(AutomatonError)` goes to `failed`, so one error type maps two ways. Fix: match the inner variants. `ComponentCertificateNotBuilt`, `FormNotDecided` and `NotSingleExistential` go to `unsupported`; the rest go to `failed` with their code. | qsl-analyze/src/analyze/items.rs:132-148 |
| FND-006 | medium | The `declined` rows lose the FR-272 cause. `TerminalRecord::declined` stores only `DeclineCode::Qsl(Code)`, the code half. The `AdmissionRecord`'s `cause` (for example `wrong-role-mapping`) is dropped, and so is the Symmetry row's `invalid-value`. FR-281's table requires "the failure's FR-272 diagnostic" and, for Symmetry, `invalid_runtime_input`/`invalid-value`; FR-280-AC-6 expects `invalid_runtime_input`/`wrong-role-mapping`. Fix: attach the full `CatalogCode` as the record's diagnostic, with a `declined_with` beside `failed_with`. | qsl-analyze/src/analyze/items.rs:110-129; qsl-analyze/src/analyze.rs:516 |
| FND-007 | medium | Every EN-4 `PredicateRefused` settles `failed` with `runtime_invariant`/`established-invariant-broken`. FR-192's rows (FR-192-AC-7) say otherwise: `KernelRefused(r)` and `FamilyRefused(code)` settle `failed` with that refusal's own code (for example `integer_out_of_domain`/`outside-domain`), and `Incomplete(i)` settles `incomplete`, `limit-reached`, naming the counter. So a predicate meter exhaustion is reported as an internal fault. The FR-192 map (`settle_statistical_refusal`) still returns `Err(PredicateRefused)` (QSL-570, SR-12360 FND-001). Either implement those rows here, or route through the map once it settles them and record on QSL-596 that `analyze` inherits the gap until then. | qsl-analyze/src/analyze/items.rs:170 |
| FND-008 | low | Spec gap for draco, and a questionable fallback. `Extent`, `Protocol`, `Translation`, `Encoding`, `Key`, `Item` and `Footprint` settle `failed` with `runtime_invariant`/`established-invariant-broken`, an internal-fault code. But `Translation` (a past operator over a future one, or too many atoms) and `Footprint` (a form FR-383 gives no rule for) are unsupported constructs, and `Item`/`Protocol` can be request defects. FR-281's table assumes each refusal has an FR-272 code; these types have none. Ask draco for the codes. Until then, prefer `unsupported_construct` for `Translation` and `Footprint`. | qsl-analyze/src/analyze/items.rs:149-157 |
| FND-009 | low | The item's work closure is bound to its adapter's engine, while the record's backend comes from the caller's disposition, and `AnalyzeItem::disposed` is public. Only the kind/mode consistency check keeps a re-disposed EN-1 item from running under another engine's `BackendId`. That cannot happen today: EN-1's and EN-4's kinds are disjoint and `Engine::new` is crate-private. It becomes a misattributed record once `Engines::with` accepts trait engines. When the `Engine` trait lands, run the engine's own `run`, or refuse a candidate other than the adapter's engine. | qsl-analyze/src/analyze.rs:314-318,410-450 |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | high | `statistical_refusal` intercepts `R::PredicateRefused` before FR-192's map (`settle_statistical_refusal`). The PR body and the QSL-596 comment of 13:01Z say the map "returns no record for PredicateRefused". That is false at #1299's current head (87620e97a): there `settle_statistical_refusal` settles every `PredicateRefused` by FR-192-AC-7's rows (`predicate_refused`: `KernelRefused` gets its own code and cause, `Incomplete` gets `incomplete`/`limit-reached`). #1299 is landing separately. Once both land, `analyze` overrides the correct map for EN-4. It returns `failed` with `runtime_invariant`/`established-invariant-broken` for a predicate meter exhaustion that should be `incomplete`, and for a kernel refusal that should carry its own code. The provider entry (`serve_request`) returns the correct record, so FR-280's analyze-equals-entry rule breaks too. Fix: delete both `R::PredicateRefused` arms and let `settled =>` call the map. Before #1299, `Err(_)` already falls back to `failed` with `BROKEN`; if needed, keep the `FamilyRefused` code under `Err(UnsettledRefusal::PredicateRefused)` only. | qsl-analyze/src/analyze/items.rs:196-206 |

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

### Round 3 (4a190547d)

FND-001 to FND-004 are already fixed. No new row is needed: `to_terminal` and `#[non_exhaustive]` are unchanged in the delta.

### Round 4 (861ef597f), final round

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 861ef597f. Inner variants are matched. Only `ComponentCertificateNotBuilt`, `FormNotDecided` and `NotSingleExistential` settle `unsupported`. Request defects settle `failed` with `invalid_runtime_input`/`invalid-value`; engine defects settle `failed` with `BROKEN`. |
| FND-006 | fixed | 861ef597f. `TerminalRecord::declined_with` keeps the full code and cause, from `CatalogCode::new(record.code.as_str(), record.cause)`; Symmetry carries `INVALID_VALUE`. |
| FND-007 | still-open | Superseded by FND-010. The "inherited gap" is caused by `analyze`'s own intercept, not by the FR-192 map, because #1299's map settles these rows. |
| FND-008 | fixed | 861ef597f. Translation and Footprint now settle `unsupported_construct`/`expression-form`. That is an existing FR-272 cause (qsl-eval/src/temporal/outcome.rs:182, qsl-replay/src/spine/bind.rs:240), not an invented one. Extent, Protocol, Encoding, Key and Item still have no spec code; that is deferred to draco (STD-444 follow-up, recorded on QSL-596 at 13:01Z). |
| FND-009 | deferred | Recorded in a `settle` doc comment. It is unreachable until the `Engine` trait lands (part 2). |
| FND-010 | still-open | This is a correctness bug: once #1299 lands it gives wrong EN-4 records and breaks analyze-equals-entry. Fix in this PR. |
