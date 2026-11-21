---
id: SR-202611210022442313144108430
title: "Code review — QSL-2128 PR #3093 model-subject hyper checks through analyze (post-rebase squash dc59305f00)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3093 diff origin/main...dc59305f00 (37 files): fsl-analyze/src/analyze{,/document,/hyper_route,/request}.rs, fsl-analyze/src/model_check/hyper{,/projected,/single}.rs, fsl-replay/src/certificate{,/product,/scc}.rs, fsl-replay/src/model_check/{existential_outcome,existential_settle,hyper_outcome,hyper_settle,settle}.rs, fsl-replay/src/outcome/counterexample.rs, fsl-eval/src/hyper/{copy_swap,mod}.rs, tests fsl-analyze/tests/it/*.rs"
review_set: subset
evaluated_revision: "dc59305f00"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-163
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-173
    type: references
---

## Summary

Ticket: QSL-2128. Code review with the rust-review lane folded in. The review covers the squashed head dc59305f00, compared against the pre-rebase head f1692df174. It also checks that #3231's event-position verdicts survive the port of `hyper_item.rs` into `hyper_route.rs`.

How the port was checked. A per-file comparison of the patch lines in the original PR (merge base 5865aa2ffe..f1692df174) and the squash (origin/main...HEAD) gives the following:
- 24 of the 38 original files are byte-identical in content.
- The rest differ only in import order, in main's per-alias `owned`/`slot_of` systems, in the `HoldsClosed { stats, .. }` arm, and in the `ClosedWitnessed` refusal.
- `hyper_counterexample_wire` and `TerminalRecord::of_hyper` were dropped from the squash because #3231 already put them on main.
- The `single()` HP-5 route, `settle_hyper_check(.., requested_limits, Some(&certificate), cancel, ..)`, `requested_limits` and the `checker`/`exploration` wiring are all present.

Nothing from the original PR was lost.

Tests run, name-filtered, at dc59305f00:
- The hyper_closed, hyper_wire, closure_certificate, hyper_settle, hyper_witness, hyper_world, hyper_replay, hyper_single, hyper_swap, hyper_projected, hyper_reductions and analyze_family_cli two-model filters: 194 passed, 7 failed, 5 ignored. The 7 failures are the known Nonzero `BodyUnproved` baseline reds.
- fsl-replay `probability_certificate::claim_binding_uses_the_checked_node_not_name_or_source_order` fails the same way (`AmbiguousName "Same"`) on a clean origin/main 3c7b98bebb worktree. It is a baseline red, not caused by this PR.

## Verdict

**NOT MERGEABLE as is (2 medium, 1 low here; see the gap-analysis SR for the high).**

What holds:
- The port is faithful.
- #3231's verdicts are untouched: every hyper_closed test passes, as do both two-model analyze tests. `HoldsClosed` still settles `proved` with no certificate, the per-alias `for_alias` systems and the bounded-body refusal are unchanged, and `decide_closed`'s `certificate: None` is sound because no closed outcome reads a certificate.
- The event-position gate is sound. No event-position item reaches the lasso/product-trap settlement:
  - HP-5 under event-position is diverted before `single()`;
  - `check_hyper` sends every other event-position form to `decide_closed`;
  - `settle_existential`'s `ClosedWitnessed` refusal is unreachable from `analyze` and refuses rather than settles.
- The FR-163 checker recomputes closure from the initial states independently and checks the components before the violation rule. The certificate errors it can produce are rejections or refusals, so no wrong `proved` was found.

The defects are completeness defects on the multi-model and event-position HP-5 paths, both reproduced in a throwaway worktree.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Multi-model hyper proofs are rejected by their own checker. The engine builds its certificates over the `for_alias` per-model systems, but the checker rebuilds over `ModelSystem::new` of the whole request. Reproduced: a two-model infinite-trace HP-2 that holds (`always holds(v.h @ a = w.h @ b)`) settles `inconclusive` `certificate-rejected` `initial-missing`, exit 22, where `proved` is expected | fsl-replay/src/certificate/product.rs:173; fsl-replay/src/certificate/product.rs:273; fsl-analyze/src/analyze/hyper_route.rs:141-154; fsl-replay/src/model_check/existential_settle.rs:38; fsl-analyze/src/model_check/hyper/single.rs:135 |
| FND-002 | medium | Event-position HP-5 through `analyze` still settles `unsupported`/`not-yet-implemented` (exit 21). Reproduced on both of FR-173-AC-6's HP-5 bodies. The gate comment says the clause "keeps its own route", but it falls through to `check_hyper`, which refuses `FormNotDecided`. FR-173-AC-6 says the HP-5 case is analyzed through native-analyze/1 and is never `not-yet-implemented` | fsl-analyze/src/analyze/hyper_route.rs:98-117; fsl-replay/src/model_check/existential_settle.rs:156-158 |
| FND-003 | low | Import and format churn in the 2024 style edition (the crate is edition 2021). It adds about 39 `cargo fmt --check` hunks beyond main's 537, in files such as settle.rs, analyze.rs, certificate.rs, scc.rs and the test imports, and reflows code the PR does not otherwise touch | fsl-replay/src/model_check/settle.rs:19-46; fsl-analyze/src/analyze.rs:56-107 |

## Findings detail

- **FND-001.**
  - The mismatch: `Run::certificates` passes `self.systems`, which `alias_system` → `ModelSystem::for_alias` builds over each alias's own model and its filtered snapshot. `ProductClosureRequest.subjects` holds the whole `simulation` once per alias, and both `complete_scope` and `check` call `ModelSystem::new` on it. In a unit that selects two or more models, the checker's initial product keys hold every model's populations, so the first initial state is missing from the certificate.
  - Why it is honest but wrong: the direction is safe (a rejection, never an acceptance), but every multi-model infinite-trace HP-2/HP-3/HP-6 proof is lost. The PR's own `hyper_wire`/`hyper_settle` tests are single-model, so none catches this.
  - The same split exists on the HP-5 path: `single()` uses the `for_alias` system for `bound_reference` but the `ModelSystem::new(alone)` system for search and settlement.
  - Fix: carry the per-alias provision in the request, or rebuild through `for_alias` with it, in `complete_scope`, `check` and `settle_existential`, and add a two-model infinite-trace proved test.
  - Lockstep product certification across several models is not in itself unsound. The defect is only the system mismatch.
- **FND-002.**
  - Soundness is fine: the refusal is honest, and the lasso settlement correctly does not handle closed witnesses.
  - This matches main before the PR, and #3231 declared the HP-5 analyze route unbuilt. It is acceptable for v1 only if it is tracked: the code comment should say it refuses, and a ticket should own the route.
  - Fix: route event-position HP-5 through `check_single_existential` (which already decides by `decide_closed`), then settle `ClosedWitnessed` and the closed `Trapped` as `HoldsClosed` and the closed replay do.
- **FND-003.** Run `cargo fmt` with the repo's own edition, or revert the hunks that only reorder.
