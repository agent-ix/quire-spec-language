---
id: SR-2700001
title: "QSL-1904 PR 2700 code-review and rust-review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1904; origin/main...3a4ac6d192303022c7695b1c61ed16a71adbd04f; qsl-semantics/src/check/{state_clause,ir,lowering,node_key/mod,check,family,state_model,sum_case,termination}.rs, qsl-semantics/src/check/lowering/timed_clause.rs, qsl-semantics/src/value/member.rs, qsl-package/src/emit.rs, qsl-package/src/emit/tests/{population_field_edge,population_bound}.rs, qsl-replay/src/spine/clause/tests.rs, qsl-eval/tests/it/relationship_reachability.rs"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1054
    type: reviews
---
# SR-2700001: QSL-1904 PR 2700 code-review and rust-review

## Summary

Ticket: QSL-1904. PR agent-ix/quire-spec-language#2700, head
3a4ac6d192303022c7695b1c61ed16a71adbd04f, one commit over origin/main.
Code review with the Rust lane (rust-review, rust-style, repo AGENTS.md)
folded in, scoped to `git diff origin/main...HEAD`, against FR-380
"Field-edge reachability" (FR-380-AC-8 to AC-21), TC-1054 rows 15-28 and the
FR-105 `reaches_field` member note.

What holds:

- `walked_object` (`state_clause.rs:740-748`) now returns `edge.owner` for a
  field-edge `Reaches`. The typer (`state_model.rs:403-427`) admits a field
  edge only when the field's reference target is `Reference(owner)` and both
  operands conform to the owner (equal under the non-complete profile), so
  `edge.owner` is the FR-322 step-2 `T`, both the owner and the target. The
  change is a no-op without `quire.model.complete/v1` and is sound under it:
  the walk visits `T` objects, which a population covering only the subtype
  need not contain. The record domain, the ambiguity refusal (still the
  field-form family, `state_clause.rs:1052-1063`) and the above-maximum
  refusal (record-derived, `state_clause.rs:1102-1108`) all follow from
  `population_of(T)`.
- The link gate (`state_clause.rs:1064-1070`) drops `end_form &&`, but only
  `Reaches` and `ReachesEnd` reach it (`walked_object` is `None` for every
  other kind) and `link_population` (`ir.rs:455-464`) writes only those two
  kinds. Projection, navigation and `allInstances` nodes get no population.
- `Member::Field.population` is `None` at every non-reaches site
  (projection, timed-clause projection, declared member), the emitter writes
  the key only when `Some` (`emit.rs:2339-2341`, absent not `null`),
  `Member::population()` now feeds the dependency visit
  (`node_key/mod.rs:1300`), and `map_member` remaps it. A `None` field adds
  no derived-key operand (`lowering.rs` `walked_operands` never reads it) and
  no wire text, so unbounded node identity is unchanged by construction.
- The population node is built over `edge.owner` (`lowering.rs:5422-5429`)
  through the shared `bounded_population`, so a field edge and an end over one
  `(T, N)` share one node and one `Type` occurrence per bound label
  (`model.rs:2636-2641`).
- Tests discriminate the main wrong implementations: bound `4` against
  sequence bounds `[0,3]`/`[2,3]`; AC-15 checks the `Set<Reference<Order>>`
  chain with `declaration` `Rush`; AC-16 fails an operand-type rule (it would
  link `rushes`); AC-21 fails an operand-type rule (it refuses ambiguous);
  AC-14 matches `CheckCause::AmbiguousName` directly, which a `StateModel`
  wrapper would not.
- AC-12's `#[ignore]` reason names QSpec STD-663 and IR-1533.
- Gates run: `cargo fmt --check` clean. Test results are recorded under
  `## Verdict`.

## Verdict

Mergeable: no finding is high or medium, and the dependent-crate runs are
green. All six findings are low: FND-001, FND-002, FND-003 and
FND-006 are test-strength gaps, FND-004 and FND-005 design/coverage notes.

Test runs (via `~/dev/worktrees/locked-build.sh`, `CARGO_PROFILE_DEV_DEBUG=0`,
`QSPEC_DIR=~/dev/quire-specification`, head 3a4ac6d19):

- `cargo test -p qsl-replay --lib`: 1244 passed, 0 failed.
- `cargo test -p qsl-semantics --lib`: 895 passed, 0 failed, 1 ignored.
- `cargo test -p qsl-package --lib`: 861 passed, 0 failed, 3 ignored
  (includes `emit::tests::population_field_edge`, AC-12 ignored).
- `cargo test -p qsl-eval --test it relationship_reachability`: 27 passed.
- `cargo test -p qsl-analyze`: 147 + 1541 + 4 passed, 0 failed.
- `cargo test -p quire-spec-language --test it -- bare_package
  config_version_spine selected_object_closure`: 23 passed, 0 failed.
- `cargo clippy -p qsl-semantics -p qsl-package -p qsl-eval -p qsl-replay
  --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The record-domain halves of AC-17, AC-18 and AC-21 assert unbounded-domain counts plus `CheckedGraph::population_bounds()` labels. `population_bounds()` is the graph-wide list of declared bounds (`check/mod.rs:8171`), so every `bound_labels` assertion passes whether or not any record names the bounded domain: it cannot fail. The counts, read across each test's bounded/unbounded pair, do fail the realistic wrong rules (operand-type walking refuses ambiguous in AC-21 and links in AC-16; dropping the walked domain gives `[1]` for unbounded FavLoop), but no test names the domains: in AC-21's rushes-bounded case a record `{people, orders(None), rushes(Some 4)}` gives the expected count `[2]`. Assert the claim record's `PopulationDomain`s (object, maximum) directly and drop `bound_labels`. | qsl-package/src/emit/tests/population_field_edge.rs:141-147, qsl-package/src/emit/tests/population_field_edge.rs:391-442 |
| FND-002 | low | AC-19 names "the clause of FR-380-AC-18" (invariant `FavLoop` over `favourite`); the test evaluates a different clause, `pre PreFav ... { true or reaches(self.fav, target, trail) }`. At 4 objects `true or` short-circuits, so "decides the clause as for an unbounded population" is shown only as `true`, never by a reaches result. The refusal half (5 objects, `[0, 4]`, `set`, `5`, no charge) is a real field-edge walked non-context population check. | qsl-eval/tests/it/relationship_reachability.rs:1605-1666 |
| FND-003 | low | AC-13's identity half compares the branch's lowering over R-plus-fields with the branch's lowering over plain R; it cannot detect a change that re-keys every field member against main, which is the failure TC-1054's description names. Identity is unchanged by construction (see Summary), so this is test strength only; a recorded pre-FR-380 key or a check that the derived-key operands exclude the member would discriminate. | qsl-package/src/emit/tests/population_field_edge.rs:233-257 |
| FND-004 | low | `Node.field_population: Option<AuthoredBound>` is a parallel field on every checked node, meaningful only on `NodeKind::Reaches`, while `ReachesEnd` carries its own `population` in `quire-checked`. The type does not enforce the Reaches-only invariant, the change edits ~15 `Node` literals, and it grows every node by an inline `AuthoredBound` (label `String` plus `DeclarationKey`), unlike the boxed `equality_producer` beside it. Sound and confined to `pub(in crate::check)`, but the symmetric design is `population` on `NodeKind::Reaches` in quire-checked (branch=main, landed together with this adaptation), or at least `Option<Box<AuthoredBound>>`. | qsl-semantics/src/check/ir.rs:79-83, qsl-semantics/src/check/ir.rs:455-464 |
| FND-005 | low | `walked_object` also feeds `walked_populations`, which refinement mapping uses (`refinement.rs:3626`, FR-146 "Claim extent"). Under complete/v1 a refinement field expression holding a field-edge `reaches` over a subtype operand now resolves `T`'s population instead of the operand's. That follows FR-380's rule, but the behaviour change on that path is untested and the PR does not mention it. | qsl-semantics/src/check/state_clause.rs:740-748, qsl-semantics/src/check/refinement.rs:3626 |
| FND-006 | low | AC-14 requires the refusal "at the `reaches`"; the test checks code, cause and name but not the refusal location. | qsl-package/src/emit/tests/population_field_edge.rs:263-283 |

## Coverage

- FR-380-AC-8 (examined): reference field `chief` linked to `Population<Order>[4]`, node in dependencies.
- FR-380-AC-9 (examined): optional reference field `trail` linked.
- FR-380-AC-10 (examined): `chain: Sequence<Reference<Order>>[0, 3]` linked.
- FR-380-AC-11 (examined): `pair [2, 3]` linked; no node of maximum 2 or 3.
- FR-380-AC-12 (examined): strict IR v2 reader admits and selects; ignored pending STD-663/IR-1533.
- FR-380-AC-13 (examined): unbounded member holds exactly kind, declaration, name; identity vs TC-1047 row 3.
- FR-380-AC-14 (examined): two populations covering `T` refuse ambiguous-name at the reaches, outside StateModel.
- FR-380-AC-15 (examined): Rush clause links the `Population<Order>[4]` node, declaration the Rush node.
- FR-380-AC-16 (examined): only `rushes` covers the operand type: no key, no node.
- FR-380-AC-17 (examined): admitted; only population domain is the context domain `rushes`.
- FR-380-AC-18 (examined): FavLoop record holds unbounded `people` and `orders` as `Population(Some(4))`.
- FR-380-AC-19 (examined): above-maximum at 5, decided at 4.
- FR-380-AC-20 (examined): field edge and relationship end share one population id and node.
- FR-380-AC-21 (examined): RushFav record holds `people`, `orders`, no `rushes`, no ambiguity.
- FR-105 `reaches_field` member note (context_only): the added `population` key only when bounded.
