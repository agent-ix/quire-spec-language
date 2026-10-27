---
id: SR-1690001
title: "code-review and rust-review of PR #1690 (QSL-1005a2, FR-159 reduction negotiation and obligation identity)"
type: SpecReview
analysis: code-review
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1690 git diff origin/qsl-1005a-zone-symmetry...7f9a8e608: qsl-foundation/src/reduction.rs, qsl-route/src/{lib.rs,reductions.rs,request.rs,model_request.rs,deadlock.rs}, qsl-route/Cargo.toml, qsl-analyze/src/{model_check/{manifest.rs,reduce.rs,request.rs},zone_check/manifest.rs}, qsl-replay/src/proof_result*, tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: reviews
---
# SR-1690001: code-review and rust-review of PR #1690 (QSL-1005a2)

## Summary

Ticket: QSL-1005. Head reviewed: 7f9a8e608, scoped to the diff against `origin/qsl-1005a-zone-symmetry` (#1671).

**What is right.**
- **`ReductionKind` in qsl-foundation.** It has one wire spelling. `constraint` round-trips, `state-constraint` spells nothing, and no `state-constraint` label remains in the wire code.
- **Registry admission.** `admit_reductions` refuses a member that is empty, holds an unknown label, or repeats one. It reports under `invalid-dynamics`, with the STD-493 note, as `memory_models` does.
- **`restrict_to_reductions`.** It keeps only the candidates that advertise every selected reduction. When that empties the set, it names the first selected reduction, in the order symmetry, partial-order, constraint, that a left-out candidate lacks.
- **Advertised sets.** EN-1 advertises `symmetry`, `partial-order` and `constraint`. EN-6 advertises `symmetry` alone and is not registered by default.
- **The schema check reads in place.** It reads `$QSPEC_DIR/proposals/backend-provider-v1/schema.json` and the checked-package schema it refers to. Nothing is copied into the repo: no backend-provider JSON is tracked. With QSPEC_DIR set, the test really ran (no "skipped" line).
- **The identity rule matches FR-159-AC-4's text.** `ObligationReductions` holds the symmetry declarations (population and classes, in request order) and the partial-order flag, never the constraint. `[[a,b,c]]` versus none gives two identities; constraint `Low` versus none gives one.

**Layering.** Moving `Reductions` and `SymmetryDeclaration` into qsl-route follows the DAG's direction. Route is layer R, which already depends on layer 3, and `quire-checked` (layer C) sits below layer 3. ADR-011's X-9 row also names `quire-checked` among route's dependencies. But route's enumerated dependency list does not include it: the §6.1 crate-map table and TC-390 both leave it out. TC-390 fails (FND-001).

**Gates.** All were run with locked-build at jobs 4, `--all-features`, debug info off, and QSPEC_DIR=qspec-e3d48d76.
- fmt: exit 0.
- qsl-analyze `reduction_manifest`: 5/5, with the schema test run, not skipped.
- qsl-analyze `reduction` / `partial_order` / `quotient` / `zone_` / `hyper_` / `statistical_`: 361/361.
- qsl-route: 18 + 77.
- qsl-replay lib: 900/900.
- clippy on the four crates with `-D warnings`: exit 0.
- **Root `family_outcome_layering` (TC-390): 9/10. FAILED.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | **TC-390 is red.** `family_outcome_layering::no_crate_below_layer_three_depends_on_the_check_core` fails with `qsl-route's [dependencies]`. Left: `[ix-trace-rs, qsl-foundation, qsl-semantics, quire-checked, quire-semantic-value, thiserror]`; right: the same list without `quire-checked`. The PR adds `quire-checked` to qsl-route so that `SymmetryDeclaration` can name its population by `DeclarationKey`. The edge points downward, and ADR-011 X-9 already lists `quire-checked` for route. But the §6.1 crate-map row ("among the workspace crates it depends only on `qsl-semantics` and `qsl-foundation`") and TC-390's list do not. Fix both in this PR: add `quire-checked` to TC-390's qsl-route list and to the §6.1 / crate-map text, citing FR-159. Alternatively, keep a population key type route already owns. | qsl-route/Cargo.toml:20-21; tests/it/family_outcome_layering.rs:485-505; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1036,1232 |
| FND-002 | medium | **One request can drop a selected constraint silently.** `author_reduced_clause` / `ModelRequest::authored_reduced` take reductions per item and deduplicate by obligation identity, which leaves the constraint out. So in one `ModelRequest`, an unconstrained item and a constrained item for the same claim collapse to one index. The kept item carries the first writer's selection: the PR's own AC-4 test writes `plain`, then `constrained`, asserts `plain == constrained` and 3 items, and the `Low` constraint is gone. This breaks FR-159 "No substitution": the engine SHALL NOT run a request without a reduction it selected. FR-159 puts `reductions` on the request (`ModelCheckRequest gains reductions`), not per item, so the spec never meets this case. Fix: carry one `Reductions` on the `ModelRequest`, or refuse a duplicate obligation whose constraint differs. Keep AC-4's two-request comparison as two `ModelRequest`s. | qsl-route/src/model_request.rs:54-77; qsl-analyze/src/model_check/request.rs:85-116; qsl-analyze/tests/it/reduction.rs:1276-1310 |

## Verdict

**Not mergeable.** FND-001 turns a merge gate red, and FND-002 can drop a selected reduction. Both are small fixes inside this PR.
