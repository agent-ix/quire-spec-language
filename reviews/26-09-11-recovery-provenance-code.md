---
id: SR-354
title: "Code review — shared recovery provenance rule for native emission and reading"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/recovery.rs; src/protocol_artifact/value_graph.rs; src/protocol_artifact/validate.rs; src/protocol_artifact/validate/control.rs; src/protocol_artifact/native/metadata.rs; src/protocol_artifact/native/runtime.rs; src/protocol_artifact/native/runtime/compensations.rs; tests/native_compensation_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

`/code-review` over `origin/main...a33a3c1` (12 files, +751/−247), Rust lane from
the actual `/home/peter/dev/agent-skills/skills/rust-review/SKILL.md` plus the
portable `/home/peter/dev/agent-skills/skills/rust-style/SKILL.md`; this repo
publishes no Rust idiom skill of its own. The increment removes the duplicated
recovery-provenance rule that SR-347 FND-001 recorded: the emitter's
span-containment scan over typed nodes is deleted, and both paths now call one
`recovery::population_members` over one `ValueGraph`. No `AssuranceProfile`
exists under this worktree (`^type: AssuranceProfile` matches nothing), so no
`## Assurance Context` section applies.

## Verdict

**CONDITIONAL** — one medium, four lows, no high. The shared rule is real, the
new test is a genuine regression, and the implicit per-`Graph` value-edge state
is gone. The one medium is the doubled read-time resolution and its missing work
vector; the rest are maintainability notes with no admitted failing case.

## Findings

| ID      | Severity | Summary                                                                                                              | Refs                                                                              | Escape Cause                    |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------- |
| FND-001 | low      | The population/closure equality predicate is written out twice — `recovery::pairs` and the model validator's pair pass check the same five fields and the same `requires == [population]` linkage | src/protocol_artifact/recovery.rs:114; src/protocol_artifact/models/populations.rs:233 | missing-requirement             |
| FND-002 | medium   | Every declaration's operand handles are now resolved and charged twice on the read path, a work cost with no dimension vector behind it | src/protocol_artifact/validate.rs:1485; src/protocol_artifact/value_graph.rs:14   | correct-requirement-no-evidence |
| FND-003 | low      | `attach` appends to a `recovery_bindings` vector already holding the chain records and sorts without deduplicating; disjointness is currently structural, not enforced at the append | src/protocol_artifact/recovery.rs:181; src/protocol_artifact/native/runtime/compensations.rs:327 | missing-requirement             |
| FND-004 | low      | `ValueGraph`'s fields are `pub` inside a `pub(super)` struct, and `control.rs` reaches the new module through an absolute `crate::` path where the file otherwise uses `super::` | src/protocol_artifact/value_graph.rs:8; src/protocol_artifact/validate/control.rs:988 | missing-requirement             |
| FND-005 | low      | The new 340-line test bundles a fixture-shape census (`forms == [5, 1, 1, 1, 2]`) with the provenance regression, so unrelated fixture edits fail the regression test | tests/native_compensation_emission.rs:181                                         | missing-requirement             |

### What the change actually establishes

One rule, one implementation. `ValueGraph::new` (value_graph.rs:14) builds the
operand/initializer/origin edges from wire handles only: operand children per
`ValueOperation`, the read binder's authored initializer, and a non-self
`Origin::Selected` edge. `population_members` (recovery.rs:10) seeds the anchor
set with the recovery anchor, traverses those edges from `recover`, and collects
reachable `Origin::Anchor` anchors. Native emission calls it from
`metadata.rs:498` after the declaration's values, binders and bindings exist;
the reader calls it from `control.rs:988`. Neither path can carry
previous-declaration state: the graph owns `owner` and `declaration` together,
`local()` (value_graph.rs:140) refuses any handle whose `declaration` is not the
owner, and `Graph::value_edges` — the implicit per-declaration field SR-347
FND-003 flagged — is deleted. `population_members` additionally re-checks with
`std::ptr::eq` that the compensation it was handed is the one at `subject`, so a
caller cannot pair a graph with a foreign compensation.

Reachability is the handle graph, not source text: the deleted emitter code
selected typed nodes by `node.span.start >= region.start && node.span.end <=
region.end`. The new test's `recordedFull` capture proves the difference — its
initializer is asserted to lie strictly before the `recover` region
(`capture.span.end < recover.span.start`) and is still followed when the
recovery reads it.

The regression is genuine. `/tmp/quire-recovery-provenance-red.log` shows the
new test failing on pre-change production at the `reads_capture = true` case with
`Invalid(Binding)` at locus `span 999..2117`. The false case is the
over-inclusion guard and was expected green before; it is honest to say only the
true direction is proven red.

The test does not use the new helper as an oracle. Expected anchors come from
`named("recoveredFull").anchor`, `full.registration_anchor` and `view.anchor`
resolved out of the retained binders, each cross-checked against the original
`scope.binders` anchor (`Anchor::Registration(0)`, `Anchor::Recovery(0)`,
`Anchor::ProtocolInstant`) and original source spans. Membership is compared as
an exact sorted `(anchor, is_closure)` vector, so a surplus or a missing pair
fails. Both branches run source → `with_proofs` → `discharged` → `native::admit`
→ `native::emit` → independent `read`, and assert `read.package() == package`.

The existing missing-member, surplus-pair and chain-substitution mutations
(native_compensation_emission.rs:1494-1610) still exercise the reader's exact
set equality and are unaffected.

Panic surface is clean: no `unwrap`/`expect`/`panic!` and no slice indexing in
either new module; every table access is `get(..).ok_or(Invalid::Reference)`.
Conversions are `u32::try_from` with a typed refusal, never bare `as` outward.
Error variants are the repo's catalogue (`Owner`, `Reference`, `Binding`,
`Duplicate`) with no string payloads. Both traversals are bounded: `visited`
caps the DFS, `Work::visit` charges `References` per inspected node and edge, and
`Dimension::Entries` is charged before each insertion.

### FND-001 — the same equality predicate, written out twice

`recovery::pairs` (recovery.rs:85-145) decides that a `Closure` belongs to a
`Population` when `closure.requires.as_slice() == [population]` and
model/value_type/subject/anchor/scope all match, then demands `paired.len() ==
populations.len()`. The model validator reaches the same members by a different
route — keying `(export.model, role.record, binding.anchor.index)` — but then
applies the same predicate to each pair at models/populations.rs:233-239:
`close.requires.as_slice() != [*population] || close.value_type != pop.value_type
|| close.model != pop.model || close.anchor != pop.anchor || close.scope !=
pop.scope` refuses, and both members have already passed `Subject::Declaration`
owner validation at models/populations.rs:156-163. A closure whose `requires` is
`[population, other]` is therefore refused on both paths, not accepted by one and
refused by the other; the earlier draft of this finding claimed such a divergence
and was wrong. No divergence between the two current predicates is demonstrated,
so this is a maintainability finding only: five field comparisons and one
linkage rule are written out in two modules, with nothing but review forcing the
next edit to touch both. Dropped to low, and the remedy is unchanged — one owner
for the predicate, or FR-042 stating it once.

### FND-002 — the read path walks the value tables twice

Before, `Graph::values` built the edges inside the type-checking loop it was
already running. Now that loop keeps its per-arm checks and `ValueGraph::new`
re-walks every value, re-resolving each operand handle through `local()` (one
`References` charge each) and re-charging `Dimension::Entries` for
`values.len()` plus one per edge. Reading a large package therefore consumes
materially more `Entries`/`References` than at `origin/main` for identical bytes,
moving the `Incomplete` boundary at a caller's chosen limits. The exact/one-short
limit test covers eight dimensions and not these two
(`tests/protocol_artifact.rs:660-706`), so no gate observes the shift.

This is a work-cost observation, not dead work: the graph is built for every
declaration because every declaration consumes it for cycle validation. At
`origin/main` `Graph::values` ended in `acyclic(&graph, self.work)`; that call is
now the last statement of `ValueGraph::new` (value_graph.rs:106), so predicates,
states and temporals still get their value-graph cycle check through it even
though only `Body::Protocol` passes the graph on (`validate.rs:1283`).
Constructing it only in the protocol arm would silently drop those checks. The
actionable remedy is to avoid the second resolution — have the validation loop
hand over the edges it has already resolved — or to give AC-9 a vector that
bounds the cost deliberately.

### FND-003 — append without deduplication

`compensations.rs:327-346` seeds `recovery_bindings` with the snapshot and the
progress/closure chain before `attach` runs; `attach` then does
`extend(members)` and `sort_unstable()` with no dedup (recovery.rs:181-183).
No overlap is admitted today, and two independent structural facts prevent one:
the chain records carry `Subject::Compensation` (compensations.rs:98-100) while
selection admits only `Subject::Declaration` (recovery.rs:96), and the chain's
`Closure` additionally carries `model: None` and three `requires`, so the exact
Population linkage `pairs` demands cannot match it. This is a prospective
maintainability note about the append site, not a defect: were a future edit to
move the chain onto the declaration subject, the producer would emit a duplicate
index and its own reader would refuse it at `intake::sorted_indices`
(control.rs:907). The right response is an explicit disjointness assertion at the
append, not silent deduplication — collapsing repeated indices would hide a
malformed identity rather than refuse it. Dropped to low.

### Gates

Root logs read, not rerun; no Cargo reproduction was needed, so
`/tmp/quire-heavy-check.lock` was not taken.

- `/tmp/quire-recovery-provenance-fmt.log` — empty.
- `-clippy-minimal.log`, `-clippy-all.log` — both `Finished`, no warning line;
  `--all-targets` with `--no-default-features` / `--all-features` and
  `-D warnings`. No new `#[allow]` appears in the diff.
- `-focused.log` — 59 cases pass.
- `-test-minimal.log` / `-test-all.log` — 55 `test result: ok` lines each, no
  `FAILED`, 4 inherited ignored, 5 doctests included;
  `recovery_population_origins_follow_used_capture_operands_not_neighboring_source`
  passes in both lanes.
- `-red.log` — pre-change production, 1 failed as described above.
- `-spec.log` — 398/398 docs grammar-clean.
- No `deny.toml` in this repo, so no `cargo deny` lane. No CI workflow file is
  touched by this diff, so no gate was narrowed.

### Dispositions of prior findings this increment touches

| Prior           | Disposition                                                                                                                            |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| SR-347 FND-001  | **Resolved.** The span-containment emitter scan is deleted; both paths call `recovery::population_members`. The residual FND-001 above is narrower still — duplicated equality text, not a divergent rule. |
| SR-347 FND-003  | **Resolved.** `Graph::value_edges` is gone; the graph is an explicit parameter from `values(owner)` to `body(owner, &values)`.           |
| SR-347 FND-002/004/005 | **Open**, untouched by this diff.                                                                                                 |
