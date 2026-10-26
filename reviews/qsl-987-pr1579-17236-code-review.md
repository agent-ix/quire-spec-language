---
id: SR-17236
title: "code-review (with rust-review lane) of QSL-987 state-graph claim node emission (quire-spec-language PR 1579)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@aabd887c597199216c54f6e77a0c0828922bcf55; git diff origin/main...aabd887c5 (PR 1579, branch qsl-987-state-graph-emit): qsl-semantics/src/check/{lowering/state_graph,state_graph_claim,mod,assemble,check,refusal}.rs, qsl-package/src/checked_v2/state_graph_root.rs, qsl-package/src/emit/tests/{state_graph_claim,admission_corpus}.rs, qsl-replay/src/model_check/{identity,graph_settle}.rs, qsl-analyze/tests/it/state_graph_settle.rs, Cargo.lock"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-410
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1140
    type: references
---

# Code review of QSL-987 state-graph claim node emission (quire-spec-language PR 1579)

## Summary

Ticket: QSL-987. Head aabd887c597199216c54f6e77a0c0828922bcf55 (open, unmerged). One pass: code-review with the rust-review lane, against FR-410, QSpec FR-475 and FR-165.

The production code is faithful to FR-410 and FR-475. Traced and probed (a temporary test over the real emit and I2 admission, reverted):

- Body shape: four bindings `kind`, `over`, `from`, `target` in order; `over` a reference to the parameter node or the empty aggregate; `from` the empty aggregate for `possible` only; no `declaration`, no `owner`; one `claim` occurrence; `semantic_type` the Boolean node. IR admits all of it (FR-322 graph shape, so `dependencies` equal the body targets).
- Identity: `possible`, `always possible` and `unique path` over equal predicates give three distinct node ids; `unique path` with `from` and `to` swapped differs; a different target differs; `c.versionNumber=2`, `(c.versionNumber = 2)` and extra spaces give the same id; reordering the claims in the unit, or adding other claims, leaves the id unchanged; a renamed claim keeps the id (the name is not in the body); the omitted `from` and `from (true)` share one id and one `true` literal node, which an explicit `{ true }` target also reuses (one literal node in the package).
- Predicate sub-terms go through `Lowering::expression` (FR-093), with slot names bound to the `over` parameter; compound units are handed to lowering.
- Requirement record: `RequirementRecord::unguarded` of `claim.requirements(key)`, Boolean type, references the objects the predicates read; keyed by the claim's `claim` occurrence; a duplicate key is an internal fault, not a silent overwrite.
- Reader: `resolve_state_graph_root` accepts only a `claim`/`state_graph` node, role `claim` and an existing ordinal; every other case is `ClauseRootUnresolved` at the triple. FR-337's lookup returns it for a state-graph triple. Strict body checks (strata and structure) are IR's admission; the QSL reader reports a body it cannot follow as `InternalFault`, which is the right layer.
- Settlement: `settle_state_graph` resolves the root first and settles `Failed` (or `Incomplete`/`Cancelled` when the emission was cancelled) before any certificate check or replay. Mutation probe: with the early return removed, `a_claim_the_request_package_does_not_hold_settles_failed` fails (the item settles `Proved`), so the test discriminates.
- Deletions: `emit_unwritten_state_graph_claims`, `with_unwritten_state_graph_claims`, `StateGraphCause::NotEmitted` and `not_emitted` have no remaining reference; the test that asserted the refusal was replaced by an identity test, the two analyze helpers and the model_gate test now compile and assert on the emitted path.
- Pin bump (quire-contract-model e31c506a, quire-verification-contracts fe19108d): the whole suite is green under it (no golden changed, conformance lane green: 21 QSpec operation vectors, 15 positive fixtures through the I2 read, 6 adverse mutations); `temporal/terminal` is correctly added to NOT_EMITTED_BY_QSL.

Gate: `make ci` stops red at `ci-default-features` on one finding (FND-001, inherited). The lanes after it were run by hand and are green (see Verdict).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Gate red, not introduced by this PR: `test-suites default-workspace` and `all-features-workspace` report `tests/fixtures/hyper-query-consumer/src/lib.rs (doctests): 0 tests passed and the lane does not declare it empty`. The fixture crate came with #1572 (QSL-1010); the PR diff touches nothing under `xtask/`, `tests/fixtures/` or the workspace manifest, and origin/main (395de2cde) is merged into the head. Fix belongs in its own ticket or PR: declare the doctest suite empty in the lane table (xtask/src/test_suites.rs WORKSPACE_EMPTY) or give the fixture a doctest. Every other lane is green. | xtask/src/test_suites.rs; tests/fixtures/hyper-query-consumer/src/lib.rs | tooling-gap |
| FND-002 | low | The cancellation arm of the new settlement ordering has no test. `ClauseRootRefusal::Cancelled` settles `Incomplete(Cancelled)` and nothing drives it with a cancelled `Cancel`, so swapping it with `Failed` passes every test. Add one test that cancels before `settle_state_graph` and asserts `Incomplete` with zero replays. | qsl-replay/src/model_check/graph_settle.rs:128-147; qsl-analyze/tests/it/state_graph_settle.rs | correct-requirement-no-evidence |
| FND-003 | low | A recompile, emission or I2 read fault of the request's package (`ClauseRootRefusal::Package`) settles `Failed` exactly like an unresolved root. FR-410 requires `Failed` only for the unresolved triple; a package that does not read back is a different fault. Defensible (the request is at fault either way, and the same mapping is used for clauses), but record the choice in FR-410 "Behavior" or split the arm. | qsl-replay/src/model_check/graph_settle.rs:140-146 | wrong-requirement |
| FND-004 | low | `settle_state_graph` now emits and re-reads the whole package once per item (`read_back` inside `StateGraphIdentity::resolve`). A request with many items over one package repeats an emission and an I2 admission per item. No correctness issue; hoist the admission into the request if item counts grow. | qsl-replay/src/model_check/identity.rs:96-117 | efficiency |
| FND-005 | info | Coder residual, AC-3 settlement half: the test observes `Failed` with zero replays rather than counting `check_state_graph` calls. Accepted: the mutation probe shows it settles `Proved` without the early return, and the closure only runs from `check_state_graph`'s success path, so a call that ran would reach the replay or a different row. A call counter would still be the literal reading of the AC. | qsl-analyze/tests/it/state_graph_settle.rs:1157-1190 | correct-requirement-no-evidence |
| FND-006 | info | Coder residual, AC-1/AC-2 fixture: the tests use equivalent predicates on the ConfigVersion domain package, not ADR-022 section 7's game units (`x.phase != G::Phase::Lost`). The body shape, node kinds and sharing rules they assert do not depend on the predicate's text, and there is no game domain package in the repo's test fixtures. Accepted. | qsl-package/src/emit/tests/state_graph_claim.rs | correct-requirement-no-evidence |
| FND-007 | info | The `over` parameter node carries the parameter's name, so renaming the parameter (`c` to `d`) changes the claim's node id while the checked meaning is unchanged. This follows from the node body FR-475 fixes and matches the temporal clause; noting it because FR-475-AC-3 says source spelling alone does not change the id. Parameter names are not "spelling" in that sense; no change asked. | qsl-semantics/src/check/lowering/state_graph.rs:57-69 | correct-requirement |

## Verdict

PASS for the production code and the tests; no blocking finding. FND-001 is a baseline gate red inherited from main, not this PR's. FND-002 should be fixed in this PR if the coder is already in the file; FND-003 and FND-004 are optional.

Gate (flock /tmp/qsl-cargo.lock make ci, once): red at ci-default-features (fmt, clippy workspace, conformance 21+2+2 vectors, 15 fixtures and the whole default workspace test run green; the lane then reports FND-001). Run by hand afterwards, all green: clippy and test-suites for qsl-semantics and qsl-cst, test-deep-format, ci-clean-build, seam-probe, string-edge, route-lint, checked-input, refinement-versioning, cargo-deny-bans, ci-docs, the three arch-lints, build-no-std, check-no-committed-binaries. ci-all-features: clippy green, test-suites all-features-workspace red on the same FND-001 finding only.
