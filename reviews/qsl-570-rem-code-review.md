---
id: SR-12360
title: "Code review (Rust lane) of QSL-570 remainder: EN-4 provider entry over the CheckedPackage (PR #1299)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bb3980366ef5d7cc687dbf73d1de57277abf0cd8; qsl-analyze/src/statistical.rs, qsl-analyze/src/statistical/provider.rs, qsl-analyze/tests/it/statistical_witness_replay.rs; context qsl-replay/src/statistical/settle.rs, qsl-analyze/src/statistical/model.rs, spec/functional/FR-280, FR-192, FR-281"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
---
# SR-12360: Code review (Rust lane) of QSL-570 remainder: EN-4 provider entry over the CheckedPackage (PR #1299)

## Summary

Ticket: QSL-570. The diff is `git diff origin/main...HEAD` at bb3980366: a new public module `qsl_analyze::statistical::provider` (`RoutedItem`, `serve_request`, `serve_items`) and one integration test. `/code-review` and `/rust-review` ran as one pass.

What was checked and found correct:

- **No `EmittedPackage`.** `serve_request` takes a `StatisticalRequest`. Its `subject.simulation.package` is `&CheckedPackage` (`qsl-eval/src/simulation/model.rs:118`). Nothing in `qsl-analyze` or `qsl-eval` names `EmittedPackage`. The entry reads the checked package only, as FR-280 Behavior 6 requires.
- **Settlement parity with existing FR-192 code.** The `Ok` arm calls `settle_statistical` with `SubjectLimits` taken from the same subject's `expansion` and `meter`, which is how every existing settlement test builds them. The `Err` arm calls `settle_statistical_refusal`. There is no second settlement map.
- **Determinism.** The record depends only on the request (seed included) and `cancel`. The test asserts that a second run returns an equal record.
- **Cancel.** `check_statistical` polls `cancel` before each draw and step. A cancelled run settles `Incomplete(Cancelled{source})` by FR-192's row. The same handle is shared across items, so after a cancel every later item settles `incomplete`. A `Deadline` cancel stands for the request's `deadline_ms`, because quire-exact's `Cancel` is owned by the frontend that holds the timer.
- **Rust idioms.** No `unwrap`, `unsafe` or panics on the production path. `meter` is copied before the request moves. The `Vec` in and `Vec` out keeps item order. fmt and clippy are fixed in bb3980366.
- **Public-API changelog.** The PR body lists exactly the three added items. The crate has no CHANGELOG file.

## Verdict

**Not mergeable as is: one high finding (FND-001).** The entry inherits FR-192 settlement that is now wrong for `PredicateRefused`. The spec change that makes it wrong (STD-282, #1256) is already in this PR's base, so this is not blocked. Fix it in this PR: one match arm group in `settle_statistical_refusal`, plus FR-192-AC-7's test. FND-002 is a doc fix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `serve_request` and `serve_items` give no terminal record for a `PredicateRefused` item. They document this and return `Err(UnsettledRefusal::PredicateRefused)`, through `settle_statistical_refusal` (settle.rs:233). FR-192 as merged in this PR's base (STD-282, #1256 merged 316359fec, an ancestor of bb3980366) settles every `PredicateRefused` (Behavior table rows 114-116, FR-192-AC-7): `KernelRefused(r)`/`FamilyRefused(code)` settle `failed` with the refusal's code; `Incomplete(i)` settles `incomplete` `limit-reached` naming the counter by FR-255 Behavior 9; `NotBoolean`/`KernelUndefined`/`FamilyUndefined`/`KernelRefused(CheckedInvariant)` settle `failed` `runtime_invariant`/`established-invariant-broken`. Only `Admission` and `RequiresBound` stay unsettled. So a routed EN-4 item whose predicate read is refused returns no record, where FR-280 requires one terminal record per routed item. The PR body calls this blocked on #1256, but #1256 is merged. Fix in this PR: settle the three row groups in `settle_statistical_refusal`, drop `UnsettledRefusal::PredicateRefused`, fix the `serve_request` doc, and add FR-192-AC-7's test (TC-1166). | qsl-replay/src/statistical/settle.rs:233; qsl-analyze/src/statistical/provider.rs:36-41; spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:114-116,152 |
| FND-002 | medium | The module doc says in the present tense that "`analyze` settles an EN-4 item through the same two calls ... so the record of an item is the same through either path (ADR-029 OP-2)". No `analyze` operation exists: FR-281 is unimplemented and its slice, QSL-596, is in Backlog. FR-280 also says the entry runs routed items through `analyze`, while this entry calls EN-4 directly. Make the doc state what is true: this is EN-4's settlement, and the `analyze` parity is FR-280-AC-2/AC-5 work for QSL-596. When QSL-596 lands `analyze`, either `analyze` calls `serve_request` for EN-4 items or `serve_items` is replaced, so that there is one path, not two. | qsl-analyze/src/statistical/provider.rs:1-11 |

## New findings (disposition pass 1)

Reviewed at 509e057d6 (fix commit 03d0e5172).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | A `PredicateRefused(Incomplete(i))` for the `work_units` counter settles `LimitReached{EvaluationMeter}`, so it is spelled `evaluation-meter` with setting `meter`. FR-192 row 115 names `i`'s counter by FR-255 Behavior 9's rule: the counter itself (`work_units`) and the setting as the counter's own name (`work_units`). FR-255 also says "The evaluation meter an engine reads is the call's accounting limits, whose setting names are the accounting table's". `meter` is no setting in any FR-255 table, so `--limit meter=...` is refused as an unknown setting and the user cannot raise the bound the record names. `ModelCheckLimit::of_counter` maps `WorkUnits` to `EvaluationMeter`, and `a_refused_predicate_read_settles_as_en5_does` asserts that wrong mapping. Fix: settle every counter as `Counter(kind)` (limit and setting `work_units`). Also fix `EvaluationMeter.setting()` to `work_units`: FR-192 row 105 says the expansion-meter row's setting is the exhausted counter's name, so `meter` is wrong there too. This is the most common meter stop. | qsl-replay/src/model_check/limits.rs:121-127,154; qsl-replay/src/statistical/settle.rs:266-271,689-695 |
| FND-004 | low | `items_with_evidence` writes `statistical` (`map_or(EvidenceKind::Statistical, ...)`) for a `probabilistic-satisfaction` record that carries no evidence kind. FR-195 says every such record carries one, so a record missing it is a broken invariant. The fallback hides it and silently routes an exact-only claim to EN-4. Make it unrepresentable (the probabilistic `Requirements` constructor takes the evidence kind), or settle it as an internal fault. | qsl-route/src/request.rs:623-631 |

## Dispositions

Round 1, by the reviewer, at 509e057d6 (fix commit 03d0e5172).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 03d0e5172. `settle_statistical_refusal` settles `PredicateRefused` through `predicate_refused` by FR-192 rows 114-116: a kernel refusal with code and cause gives `failed` with that code (only `CheckedInvariant` has none, so it gives `BROKEN`); a family refusal gives `failed` with its code; `Incomplete` gives `incomplete` `limit-reached` with the configured bound; `NotBoolean` and the undefined outcomes give `BROKEN`. `UnsettledRefusal::PredicateRefused` is deleted, and the FR-192-AC-7 test is added. The counter-naming part is wrong; see new FND-003. |
| FND-002 | fixed | 03d0e5172. The module doc now says that no `analyze` exists (FR-281, QSL-596), and that `analyze` must settle through `serve_request` so the two stay one path. |
