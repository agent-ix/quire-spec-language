---
id: SR-1953001
title: "Code review of PR #1953 (QSL-1340): FR-411 deadlock verdict joined on the terminal declaration"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1953 git diff origin/main...2282c6a49 (branch qsl-1340-fr411): qsl-semantics/src/check/terminal.rs, qsl-route/src/{deadlock,model_request,request}.rs, qsl-replay/src/model_check/replay.rs, qsl-analyze/src/analyze/items.rs, qsl-analyze/tests/it/{derived_request,dense_route,zone_replay}.rs; ticket QSL-1340"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-411
  type: reviews
---
# SR-1953001: code review of PR #1953 (QSL-1340)

## Summary

Ticket: QSL-1340. PR: quire-spec-language#1953, head 2282c6a49. Method: code-review with the rust-review lane. The PR records each model alias's terminal node key on a model subject's derived deadlock-freedom request item. It compares those keys in the EN-1 and EN-6 derived-item adapters and in a wrapper around the counterexample replay. The EN-1 path is sound: the run package, the certificate package and the replay's recompiled package are all compared. The EN-6 path is not. It compares only the certificate package, and EN-6 has a proof row that never runs the certificate checker.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| TerminalKeys, TerminalMismatch, CheckedGraph::terminal_keys | qsl-semantics/src/check/terminal.rs:278-362 | examined |
| SubjectReading.terminals and merge | qsl-route/src/deadlock.rs:211-285 | examined |
| derived_item terminals, derived_terminals | qsl-route/src/request.rs:279-345,584-618; model_request.rs:248-260 | examined |
| ModelTraceRefusal::TerminalDeclaration, ModelTraceReplayResult.terminals | qsl-replay/src/model_check/replay.rs:276-280,660-666,1032-1040 | examined |
| join_terminals, joined_replay, derived_model_item, derived_zone_item | qsl-analyze/src/analyze/items.rs:288-338,378-415,504-560 | examined |
| resolve_terminals (key source agreement) | qsl-package/src/checked_v2/terminal.rs:200-250 | context_only |
| ModelSystem::terminal_class (reads every alias) | qsl-eval/src/simulation/model/terminal.rs:72-101 | context_only |
| settle_model_check / settle (EN-1 rows) | qsl-replay/src/model_check/settle.rs:222-420 | context_only |
| settle_zone_check HoldsReduced row | qsl-analyze/src/zone_check/settle.rs:153-262 | context_only |
| EN-6 symmetry over DeadlockFreedom | qsl-analyze/src/zone_check/search.rs:470-600 | context_only |
| zone certificate checker terminal reads | qsl-replay/src/certificate/zone_certificate.rs:960-990,2160-2165 | context_only |
| analyze / run / settle (request-index path) | qsl-analyze/src/analyze.rs:541-720 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `derived_zone_item` joins only `item.certificate.simulation.package`, never the package of the EN-6 run's `TimedSystem`. That is safe only when every `Proved` row goes through the zone certificate checker. It does not. For `ZoneCheckItem::DeadlockFreedom`, `reads_unreduced_graph` is false, so a selected symmetry reduction runs reduced. That yields `ZoneCheckOutcome::HoldsReduced`, and `settle_zone_check` settles that as `Proved{Reduced, Uncertified}` without calling `check`. A timed deadlock-freedom run over a package with another terminal declaration (for example none instead of `when`) therefore settles `proved` while the certificate package matches. Fix: give `ModelSystem` a `package()` accessor (the field already exists at qsl-eval/src/simulation/model.rs:443) and join `item.request.subject.system().package()` as well, as EN-1 does. | qsl-analyze/src/analyze/items.rs:528-531; qsl-analyze/src/zone_check/settle.rs:178-190; qsl-analyze/src/zone_check/search.rs:515-519,591-594 |
| FND-002 | medium | The join lives only in the `derived_*` adapters. The public `model_item` and `zone_item` still build a `ModelCheckItem::DeadlockFreedom` / `ZoneCheckItem::DeadlockFreedom` item at any `RequestIndex`, and `analyze` settles it by index without reading the `RequestItem`. So a caller can settle a deadlock-freedom request item with no terminal join at all. FR-411 now says QSL settlement SHALL own this join. Either refuse a DeadlockFreedom run in `model_item`/`zone_item`, or restrict them to authored items, so the derived adapters are the only route. This bypass predates the PR for the other FR-368 joins, but FR-411's new SHALL makes it a soundness claim. | qsl-analyze/src/analyze/items.rs:87-112,215-250; qsl-analyze/src/analyze.rs:541-628 |

## Verdict

**Changes requested (one high).** I checked these and found them sound.

- Key correspondence. `CheckedGraph::terminal_keys` and `resolve_terminals` both take the node from `CheckedTerminal::occurrence().node()`, so the join key is the node the classifier resolved. The node is content-keyed. A reviewer probe found two different `when` predicates (`self.value = 3` and `= 2`) give two different keys. A duplicate declaration on one alias has no occurrence on its second entry, so `resolve_terminals` refuses and no run is built.
- Every alias. `ModelSystem::terminal_class` reads every entry of `terminals`, whatever model it is on, so recording every alias is required, not over-strict.
- EN-1. The run, certificate and replay packages are all compared. `HoldsReduced` on EN-1 is covered because the run package is joined.
- Replay. `replay_on` stamps the recompiled package's keys, and `joined_replay` maps a mismatch to `ModelTraceRefusal::TerminalDeclaration`. That is `StaleDependency`, so the result is `Inconclusive(ReplayRefused)` and never `Refuted`, on both EN-1 and EN-6.
- Time-lock-freedom reads no declaration (`over_time.rs:256-272` decides `TimeLock` before the classifier), so it correctly carries no `terminals`.
- Gates. The focused tests pass: qsl-route (21), qsl-replay model_check (77+7), and qsl-analyze `--test it` derived_request/deadlock_item/model_settle/zone_ (139). Clippy `--all-targets --all-features -D warnings` is clean on the four crates, and `cargo fmt --check` is clean.

## Dispositions

Round 1, fix commit `cfe66261f`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `ModelSystem::package()` added; `derived_zone_item` joins `item.request.subject.system().package()` and the certificate package. `a_timed_deadlock_verdict_is_joined_on_the_run_and_check_packages` refuses a symmetry-reduced run over the `when { true }` package for an item written with no declaration; dropping the run package from the join fails it. |
| FND-002 | fixed | `model_item`, `protocol_item` and `zone_item` return `Result` and refuse a deadlock-, race- or time-lock-freedom run with `DerivedItemMismatch::DerivedRun`; the derived adapters build through private `run_*` functions, so no public path settles a derived item without its join. TC-762's deadlock tests now run through `derived_model_item`. |
