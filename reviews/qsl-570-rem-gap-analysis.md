---
id: SR-12361
title: "Gap analysis of QSL-570 remainder against FR-280-AC-1/2/4/5, TC-761, FR-281/TC-762, FR-192, FR-195 (PR #1299)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@bb3980366ef5d7cc687dbf73d1de57277abf0cd8; spec/functional/FR-280 (AC-1, AC-2, AC-4, AC-5), spec/test-cases/TC-761, spec/functional/FR-281 + TC-762 (context), FR-192 (AC-7), FR-195 (AC-6), FR-075 (AC-18), TC-1163, TC-1166; qsl-analyze/src/statistical/provider.rs, qsl-analyze/tests/it/{statistical_witness_replay.rs,statistical_manifest.rs}, qsl-route/src/request.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-761
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
---
# SR-12361: Gap analysis of QSL-570 remainder against FR-280-AC-1/2/4/5, TC-761, FR-281/TC-762, FR-192, FR-195 (PR #1299)

## Summary

Ticket: QSL-570. Planless. Plan completion: not assessed. Traceability comes from the trace tags. The tagged test `the_provider_entry_serves_a_package_s4_refuses_to_emit` was run at bb3980366 (see Verdict).

How the tests bind to the criteria:

- **FR-280-AC-5, `the_provider_entry_serves_a_package_s4_refuses_to_emit`.** The binding is partial. It covers: `Service`'s `emit_checked` refused (cause only) before and after; one item served over the checked package; `measured`/`rejected`; three witnesses, each `Reproduced`; the EN-4 `BackendId`; index 0; equal on a re-run. It does not cover the record equal to `analyze`'s, the witnesses equal to `analyze`'s, the catalog code `unsupported_construct`, or a request carrying evidence kind `statistical` (FND-001). The `analyze` half is blocked, because no `analyze` exists (FR-281 is QSL-596, Backlog). The code and evidence-kind parts are not blocked.
- **FR-280-AC-4.** Already bound by `statistical_manifest.rs` (#1204). Unchanged.
- **FR-280-AC-1.** Not bound. It needs each engine's manifest and TC-762's settled kinds, so it depends on FR-281 (QSL-596).
- **FR-280-AC-2.** Not bound. It needs `analyze` (QSL-596).
- **FR-192-AC-7.** No test, and the code does not do it: see SR-12360 FND-001.
- **FR-075-AC-18 and FR-195-AC-6 (TC-1163).** No test and no code. Neither `RequirementItem` nor `RequirementRecord` carries an evidence kind (FND-003).

How each blocker the PR body claims was measured:

- **FR-280-AC-2, and AC-5's "equals analyze's record": truly blocked.** No `analyze` operation exists in the repo. The FR-281 slice is QSL-596 (LC8, "analyze op: settlement, replay, FR-280 provider entry, engine manifests, canonical request"). It is Backlog, and every blocker it lists is Done (QSL-489, 592, 593, 490, 526; STD-143). So it is ready, but it is a separate slice and QSL-596 owns FR-280 as a whole.
- **FR-280-AC-1: truly blocked** on QSL-596, as above.
- **The FR-331 provider-request type and PV-3 `Provider::run`: not QSL work.** FR-280's Overlap section gives the `Provider` trait and its adapter to the driver repository. Nothing in QSL or its dependencies defines a provider-request type. Routing items as `StatisticalRequest`s is a valid QSL-side shape. This is not a QSL-570 item.
- **The request builder's evidence kind: NOT blocked.** STD-290 is Done, and its spec (#1249, 6fb751a31) is in this PR's base. FR-075-AC-18, FR-195-AC-6 and TC-1163 are now specified. The Linear "SPEC GAP" comment of 14:56Z is stale.
- **EN-4 `PredicateRefused` settlement: NOT blocked.** #1256 (STD-282) is MERGED, as 316359fec, which is an ancestor of the PR head.
- **FR-189-AC-15's `Quantity<ms>` parameter clause: still blocked.** STD-113 is Done, but the fixtures (`prob_world.rs`, `probabilistic_declarations.rs`) record that FR-120 has no encoding for a quantity value. This review found no ticket for that gap.
- **FR-185-AC-9: truly blocked** on QSL-971 (Backlog).
- **FR-075 mutation runs: not blocked.** Nothing prevents running them. This is not an acceptance criterion.
- **QSL-965 (Specced, "EmittedPackage vs probabilistic package").** On the QSL side it is resolved by the merged FR-280 text together with this PR's `CheckedPackage` entry. What remains is the driver adapter.

## Verdict

**Gap analysis: no high finding of its own.** The high is SR-12360 FND-001. The test is real: an actual seeded model, an independent replay oracle, and no tautology. But it exercises only the `Completed` arm of `serve_request`, and its AC-5 tag overstates what it covers. Test run: `cargo test -p qsl-analyze --test it -- statistical_witness_replay` at bb3980366, 9 passed, including `the_provider_entry_serves_a_package_s4_refuses_to_emit`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The test is tagged FR-280-AC-5 but covers only part of it. Equality with `analyze`'s record and witnesses is blocked (QSL-596). Two clauses are not blocked and are missing: (a) the assert checks only the refusal's cause `not-yet-implemented` and not its code `unsupported_construct`, and should check both; (b) the routed item carries no evidence kind `statistical`, which depends on FND-003. Note on QSL-570 that FR-280-AC-5 stays open although a test is tagged, so the matrix count does not close it. | qsl-analyze/tests/it/statistical_witness_replay.rs:470-525 |
| FND-002 | medium | The test is too weak to catch a wrong settlement or a wrong ordering in the entry. It serves one item at index 0 that completes. All of the following would pass it: a `serve_items` that wrote `RequestIndex::new(0)` for every item, reversed the items, mis-settled any refusal (`settle_statistical_refusal` is never reached), or passed the wrong `SubjectLimits` (no expansion stop occurs). Add a multi-item case: index 0 `Two` completes; index 1 with a method that does not decide the form returns `Err(NotStatistical)`; index 2 with `max_draws` small enough stops `incomplete` `limit-reached`; a pre-cancelled `Cancel` gives `Incomplete(Cancelled)`. Assert each record's `request_index` and value. | qsl-analyze/tests/it/statistical_witness_replay.rs:470-525; qsl-analyze/src/statistical/provider.rs:42-74 |
| FND-003 | medium | The request builder writes no evidence kind on a probabilistic-satisfaction item. FR-075-AC-18 and FR-195-AC-6 (TC-1163, merged via STD-290 #1249) require S3 to record each probabilistic claim's evidence kind on its requirement record, and `items_from_requirements` to write items with it: one item per selected kind, `P95` `statistical`, `FairTerminates` `exact`. `RequirementItem` and `candidates_of` have no evidence kind, so QSpec FR-290 would refuse every such item `absent-evidence-kind`. This is implementable now and is QSL-570 remaining item 2. Do it in this PR or in a QSL-570 follow-up PR before the ticket closes. | qsl-route/src/request.rs:543-596 |

## New findings (disposition pass 1)

Reviewed at 509e057d6 (fix commit 03d0e5172).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | `each_probabilistic_claim_records_its_evidence_kind` is tagged FR-195-AC-6, but it builds each record by hand (`RequirementRecord::model_claim(world.claim(name).requirements(node(fill)), node(99))`). Its kind, extent, result-bound and path assertions then check what the test itself passed in. S3 inserts no probabilistic record into `CheckedGraph::requirements()`: the coder records this as a gap, and `check/mod.rs` keys records only for lowered nodes. So an S3 that writes nothing passes the test, and on a real package the request builder sees no probabilistic item. Only the `exact_only` to evidence-kind classification is really tested. The identity-equality clause (QSpec FR-407-AC-6) is not asserted either. Fix in this PR: tag the test `TC-1163` only, as was done for FR-280-AC-5, and record FR-195-AC-6 as open. Ticket the S3 work. ADR-012 says "after lowering, S3 keys each claim by its site's occurrence key", but no probabilistic claim is lowered to a checked-graph node and no spec found gives that node's form. That makes it a spec question first (STD), then code under QSL-570. | qsl-analyze/tests/it/probabilistic_request_items.rs:47-90; qsl-semantics/src/check/mod.rs:3133-3166 |

## Dispositions

Round 1, by the reviewer, at 509e057d6 (fix commit 03d0e5172).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 03d0e5172. The FR-280-AC-5 tag is dropped (now `#[trace("TC-761")]`, with the doc naming the part that holds without `analyze`), and the refusal is asserted as `(Code::UnsupportedConstruct, Some("not-yet-implemented"))`. The analyze-equality and evidence-kind clauses of AC-5 remain open under QSL-596. |
| FND-002 | fixed | 03d0e5172. `the_provider_entry_keeps_each_items_index_and_settlement` serves indices 7, 3 and 5 out of order: `max_samples` 10 gives `incomplete` `limit-reached{MaxSamples, 10}`, Regenerative gives `Err(NotStatistical)`, and the third gives `measured`. `a_cancelled_run_settles_each_item_cancelled_in_order` checks `Incomplete(Cancelled{Requested})` at 4 and 2, in order. A mis-indexed, reordered or refusal-swallowing `serve_items` now fails. |
| FND-003 | fixed | 03d0e5172. `Requirements` carries `ClaimEvidence`; `CheckedProbabilisticClaim::requirements` records `exact` when `exact_only` and `statistical` otherwise; `items_with_evidence` with `EvidenceSelection::{Record, One, Both}` writes the items per FR-075 (both kinds at indices `n + j`, record indices unchanged, no kind on other capabilities); `candidates_of` routes by `candidates_for_evidence`. `the_request_builder_names_the_evidence_kind_of_each_probabilistic_item` checks the FR-075-AC-18 results. The S3-record half is new FND-004. |
