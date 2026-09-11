---
id: SR-347
title: "Code review — source-owned compensation emission and reader identity chains"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/native/runtime.rs; src/protocol_artifact/native/runtime/compensations.rs; src/protocol_artifact/native/layout.rs; src/protocol_artifact/native/controls.rs; src/protocol_artifact/native/metadata.rs; src/protocol_artifact/validate.rs; src/protocol_artifact/validate/control.rs; tests/native_compensation_emission.rs; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Recheck of the `/code-review` findings previously recorded at `c7275f5`, now
against the frozen correction commit `b7aafe1`, using the Rust lane at
`/home/peter/dev/agent-skills/skills/rust-review/SKILL.md` and the portable
`rust-style` defaults (this repo publishes no Rust idiom doc of its own). This
is a finding recheck over the `c7275f5..b7aafe1` diff plus the code it touches,
not a re-review of the whole increment. All four previously open code findings
are resolved and demonstrated: the effect identity gate now dispatches on
`kind`, the recovery half of the chain is fully pinned, the typed-effect lane is
tested in both directions, and the reference pass restores the declaration
locus. What replaces them is narrower: the reader now reconstructs the expected
recovery inventory by a second, structurally different algorithm from the one
the emitter used to build it.

## Verdict

**CONDITIONAL** — two mediums, three lows, no high. The corrections are real and
mutation-tested; the residuals are a duplicated attribution rule that can refuse
a genuine package and a set of newly load-bearing refusals with no adverse case.

## Findings

| ID      | Severity | Summary                                                                                                          | Refs                                                                                                  | Escape Cause                        |
| ------- | -------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FND-001 | medium   | The expected `recovery_bindings` inventory is now computed twice by different algorithms — emitter by typed-node span containment, reader by the validated value graph — and compared for exact equality | src/protocol_artifact/native/runtime/compensations.rs:462; src/protocol_artifact/validate/control.rs:986 | missing-requirement                 |
| FND-002 | medium   | The clock's activation edge and temporal-profile authority, the snapshot's type/model and recovery-anchor back-reference, and the effect's retry-anchor checks are newly load-bearing with no adverse case | src/protocol_artifact/validate/control.rs:686; src/protocol_artifact/validate/control.rs:908           | correct-requirement-no-evidence     |
| FND-003 | low      | `Graph::value_edges` is implicit per-declaration state; `compensation_recovery` reads the right declaration's graph only because `values(owner)` precedes `body(owner)` in one driver loop | src/protocol_artifact/validate.rs:179; src/protocol_artifact/validate.rs:1127; src/protocol_artifact/validate.rs:1522 | missing-requirement                 |
| FND-004 | low      | `binding.scope != snapshot.scope` on progress/closure cannot fail: `compensation_subject` already pins scope index 0 and `local()` pins the declaration | src/protocol_artifact/validate/control.rs:975                                                          | correct-requirement-no-evidence     |
| FND-005 | low      | Compensation anchors have the lowest precedence in `evaluation_anchor`; four later loops overwrite them unconditionally (unchanged from `c7275f5`) | src/protocol_artifact/native/layout.rs:734                                                             | missing-requirement                 |

## Dispositions of the SR-347 findings recorded at c7275f5

| Prior   | Disposition                                                                                                                                |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| FND-001 | **Resolved.** `validate.rs:620` dispatches on `kind` alone. `compensation_effect_identity` runs the full operation/subject/slot/attempt/retry-anchor chain first and only then returns `Unsupported::Export` for a non-null type. Red-then-green: `/tmp/quire-native-compensation-review-red.log` fails this exact assertion on the pre-correction build. |
| FND-002 | **Resolved.** `compensation_recovery` (control.rs:898) pins clock, snapshot, progress, closure and exact `recovery_bindings` membership. Residual is FND-001/FND-002 above. |
| FND-003 | **Resolved.** `adding_a_payload_type_cannot_bypass_compensation_effect_authority` covers both directions; TC-121 step 6 now names the mutations. |
| FND-004 | **Resolved.** `compensations.rs:44` sets the locus before `visit()` and restores the declaration locus after the loop; an early `Err` correctly keeps the control locus. |
| FND-005 | **Open**, unchanged — `layout.rs` is untouched by `b7aafe1`. Carried above as FND-005. |

### FND-001 — one rule, two implementations

The emitter builds `recovery_bindings` in `compensations.rs:437-523`. It seeds
the anchor set with the recovery anchor, then walks `context.typed.nodes()`
selecting nodes by **source-span containment** inside the `recover` expression's
region, following `ObservationOrigin::Selected` into further expressions, and
collecting `ObservationOrigin::Anchored` anchors. It then admits a binding as a
population member when

```rust
kind == Population || (kind == Closure && model.is_some())
```

and, for `Closure`, additionally requires `requires` to be exactly one index
naming a `Population` binding.

`recovery_origins` (control.rs:1021) reimplements the traversal over the
already-validated `value_edges` operand/initializer/origin graph — which the
review request correctly describes as the better reachability relation — and
`compensation_recovery` reimplements the membership predicate as

```rust
Population => true,
Closure => model.map(|m| self.export(m)?.kind == ExportKind::Population)
```

with no constraint on `requires`. Neither half matches its counterpart: a
`Closure` whose model is not a Population export but whose single prerequisite
is a Population binding is a member for the emitter and not for the reader; a
`Closure` with a Population-kind model and a different `requires` shape is a
member for the reader and not for the emitter. Likewise the two anchor sets are
derived from different graphs, so one can reach an `Origin::Anchor` node the
other does not.

Because `compensation_recovery` then demands exact set equality
(`recovery_bindings.len() != required.len()` followed by a zipped comparison),
any divergence is not a permissive gap — it refuses a package the compiler in
this repository just produced. Scenario: a `recover` predicate that references a
`let`-bound value whose typed node lies outside every expression region the
emitter walks, but which is an operand edge in `value_edges`. The reader adds
that value's capture anchor, requires its population pair, finds it absent from
`recovery_bindings`, and returns `Invalid::Binding` on a genuine emission.

Derived from the code, not executed: constructing the divergent input needs a
new source fixture and a new test, and this recheck makes no code, spec or test
edits. The single authored `RecoveryFlow` fixture is the only shape under which
the two algorithms are known to agree.

The remedy is a single owner for the rule — either the reader consumes the
emitter's construction as data it re-derives from one shared helper, or FR-042
states which construction is authoritative and the other validates against it.

### FND-002 — what the new refusals do and do not prove

Load-bearing and mutation-tested at `b7aafe1`: the clock selection (both the
one-sided and exchanged swaps), cross-obligation substitution of Snapshot,
Progress and Closure, removal of each genuine Full population/closure member,
insertion of the genuine Partial-only pair sorted and unique, and each of the
seven prerequisite edges (snapshot→activation, progress/closure→clock, →effect,
→snapshot) severed singly. Each mutation is preceded by an assertion that the
original edge or member exists exactly once, so no case rewrites nothing and no
assertion is skipped by a false guard.

Not exercised, though now capable of refusing: the clock's own
`requires == [activation]` edge; `definition_binding(clock, definitions[profile])`,
which is the only tie between the clock and the selected temporal profile's
contract and authority; the clock's null `value_type`/`model`; the snapshot's
`value_type`/`model` agreement with the recovery binder's `Record`/`Object`
export; `anchor.kind == Recovery`, `anchor.owner == subject` and
`anchor.binding == snapshot`; the effect's `attempt.anchor`, `attempt.model`,
`anchor.kind == Retry`, `anchor.owner` and `anchor.binding` checks; and
`recovery_origins`' refusal of a phase anchor owned by another obligation.
These go beyond what TC-121 step 6 asks for, so this is a test-completeness
residual rather than a spec-alignment defect — but under FND-001 an over-strict
condition here is exactly the kind that refuses a genuine package silently.

### What the corrections establish

- Three new tests, ten compensation tests total, all passing
  (`/tmp/quire-native-compensation-corrections-focused.log`).
- Two of the three are genuine regressions: `/tmp/quire-native-compensation-review-red.log`
  records `7 passed; 2 failed` on the pre-correction build, failing at
  `native_compensation_emission.rs:1036` (`Some(Unsupported(Export))` expected,
  `None` observed) and `:1145` (`Some(Invalid(Binding))` expected, `None`
  observed). That is the previous FND-001 and FND-002 reproduced before the fix.
- The third, `compensation_attempt_bound_preserves_signed64_maximum_and_refuses_one_beyond`,
  is not in the red log and is not a regression test: `i64::MAX` attempts were
  already accepted at `c7275f5`. It is a characterization test of the authored
  bound, and its refusal half exercises the existing wire decoder. Recorded
  honestly rather than counted as red-then-green evidence.
- That test does run a real source→`admit`→`emit`→independent `read` case: it
  asserts `Full` retains `i64::MAX` and `Partial` retains `3`, compares the
  original authored token `"9223372036854775807"` and its source slice from the
  namespace syntax, and asserts `read.package() == package` and digest equality.
  Resealing `9223372036854775808` refuses with
  `NumberError::ComponentOutOfRange { component: Decimal }`. No feature ceiling
  and no numeric engine was added; the upper bound is the existing signed-64
  decoder.
- The effect identity chain is now checked twice — once binding-first from
  `validate.rs:620` and once obligation-first from `compensation_bindings`. That
  is deliberate rather than redundant: the binding-first path is the only one
  that catches a `CompensationEffect` binding no compensation names. Noted, not
  filed.
- Seam compliance unchanged: no `#[cfg(test)]` behaviour branches, no test-only
  feature flag, no forged admission report. The `serde_json` reseals are
  decoder-boundary probes of the public reader.

The positive evidence recorded for `c7275f5` stands unchanged and is not
restated: every positive case still runs actual source through `with_proofs` →
`discharged` → `native::admit` → `native::emit` → independent `read` with
`read.package() == package` and digest equality, and the adverse cases still
assert exact typed causes and source slices rather than `is_err()`.

### Rust idioms and panic surface in the correction

The correction adds no `unwrap`/`expect`/`panic!` on a library path. Direct
indexing appears at `control.rs:1041` (`declaration.values[index]`),
`control.rs:1063` (`self.value_edges[index]`) and `control.rs:681`
(`compensations[selected]`); each index is first resolved through `local()`,
which pins `handle.declaration == owner` and bounds-checks against the same
declaration's table, and `value_edges` is sized from `declaration.values.len()`
in `values()`. Safe, and consistent with the file's existing idiom of
`&self.package.declarations[owner]`. `recovery_origins` is bounded by a
`visited` set over the declaration's values and charges `Entries` per visited
node and per pushed edge before the work, so the DFS has a ceiling.

Error variants are the repo's typed catalog throughout — `Invalid::Binding`,
`Invalid::Reference`, `Unsupported::Export` — with no string payloads. The
`registered_binding`/`definition_binding` split removes the duplicated
definition-scan body rather than adding one. `definition_binding` re-checks
`definition.artifact != binding.contract`, which is already true on the
`registered_binding` path and is the real check on the clock/profile path; it is
a cheap guard, not a finding.

`quire coverage` now raises the same `oracle-resembles-implementation`
suspicion five times instead of four (the `integer` helper, similarity 0.86,
against `integer_value` in `tests/native_protocol_emission.rs`); the new
occurrence is the attempt-bound test using the same helper. Same item, not a new
class, and the helper only unwraps a checked wire number for comparison against
authored literals.

### Gates inspected

Frozen root logs read, not rerun; no new Cargo reproduction was needed, so
`/tmp/quire-heavy-check.lock` was not taken.

- `/tmp/quire-native-compensation-corrections-fmt.log` — empty.
- `/tmp/quire-native-compensation-corrections-clippy-minimal.log` and
  `-clippy-all.log` — both `Finished`, no warning lines.
- `/tmp/quire-native-compensation-corrections-focused.log` — 58 cases, all
  passing: 10 `native_compensation_emission`, 5 `native_population_emission`,
  15 `native_protocol_emission`, 4 `native_query_emission`, 24
  `protocol_artifact`.
- `/tmp/quire-native-compensation-corrections-test-minimal.log` — 55 `test
  result` lines, **557 passed, 0 failed**, 4 ignored.
  `-test-all.log` — 55 lines, **573 passed, 0 failed**, 4 ignored (3 ×
  `fixture_audit` IT-004, 1 × `required_generated_activation_parity` LC04). Both
  include the 5 `Doc-tests quire_spec_language` compile-fail cases.
- `/tmp/quire-native-compensation-review-red.log` — pre-correction baseline,
  `7 passed; 2 failed`.
- No `deny.toml` in this repo, so no `cargo deny` lane.
- `quire coverage --scope /home/peter/dev/worktrees/quire-language-native-compensation --json`
  re-run here: 367/376 backed, 0 status lies, 20 untracked symbols, 3 unmatched
  tags — byte-identical to `c7275f5` apart from the fifth suspicion above.

No `AssuranceProfile` is installed in this repository — `^type: AssuranceProfile`
matches nothing under the worktree — so no `## Assurance Context` section
applies.
