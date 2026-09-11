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

`/code-review` with the Rust lane from `/home/peter/dev/agent-skills/rust-review/SKILL.md`
and the portable `/home/peter/dev/agent-skills/rust-style` defaults (this repo
publishes no Rust idiom doc of its own), over `agent-a/native-compensation-emission`
at `c7275f5` diffed against `origin/main` (which already carries query PR56 and
producer PR55). The compiler lowers actual parsed compensation declarations —
registration after the selected forward effect, both capture stages, trigger
guard, bounded retry identities, exact operation/role, commit/never, and the
authored full/partial recovery predicates — from admitted reports only, and the
seven new tests read emitted bytes back independently. The narrow payload-free
effect contract is enforced, but only in the branch where `value_type` is null:
supplying any value type on a compensation effect skips the identity gate
entirely, and the compensation clock/recovery authorities have no reader-side
identity check at all.

## Verdict

**FAIL** — FND-001 is high: the reader's compensation-effect identity gate is
keyed on `value_type.is_none()`, so a resealed package that adds a type can name
any model on the effect binding. Everything else in the increment is sound; the
fix is small and local (gate on `kind`, not on `value_type`).

## Findings

| ID      | Severity | Summary                                                                                                          | Refs                                                                                                  | Escape Cause                        |
| ------- | -------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FND-001 | high     | A typed compensation effect bypasses `compensation_effect_identity`; nothing then ties its model to the compensation's operation export | src/protocol_artifact/validate.rs:620; src/protocol_artifact/validate/control.rs:668; src/protocol_artifact/validate/control.rs:831 | implementation-bug-despite-evidence |
| FND-002 | medium   | Compensation clock, snapshot, progress, closure and `recovery_bindings` membership are accepted on kind alone — no subject, anchor or prerequisite check | src/protocol_artifact/validate/control.rs:863; src/protocol_artifact/validate/control.rs:911; src/protocol_artifact/validate/control.rs:641 | correct-requirement-no-evidence     |
| FND-003 | medium   | TC-121 step 6 claims an explicitly typed effect view retains its payload/model fields; no test exercises that lane in either direction | spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:94; tests/native_compensation_emission.rs:600 | correct-requirement-no-evidence     |
| FND-004 | low      | `work.locus` is left on the last visited control after the compensation reference pass, so later failures in the same declaration report a foreign locus | src/protocol_artifact/native/runtime/compensations.rs:45                                               | missing-requirement                 |
| FND-005 | low      | Compensation anchors have the lowest precedence in `evaluation_anchor`; four later loops overwrite them unconditionally | src/protocol_artifact/native/layout.rs:734                                                             | missing-requirement                 |

### FND-001 — the identity gate is keyed on the wrong field

`validate.rs:620` reads:

```rust
if binding.kind == BindingKind::CompensationEffect && binding.value_type.0.is_none() {
    self.compensation_effect_identity(owner, index, binding)?;
} else if (binding.value_type.0.is_none() || binding.model.0.is_none()) && !matches!(...) {
```

`compensation_effect_identity` is the only place that asserts
`binding.model == compensations[selected].operation` and
`export(&value.operation, &[ExportKind::Operation])`. In the `else` branch a
`CompensationEffect` with a non-null type only has to have *some* non-null
model, and `self.export(model, &[])` at `validate.rs:618` passes an empty kind
filter, so any export of any kind is accepted. `compensation_bindings` then
checks the effect binding's subject, anchor, `requires == [attempt_instance]`
and its ObservationBinding contract (`control.rs:831`) — but never its model.

Concrete scenario, the same shape the suite already builds at
`tests/native_compensation_emission.rs:600`: take the emitted `RecoveryFlow`
package, set `declarations[flow].bindings[Full.effect_instance].value_type =
Some(full.attempt_type)` **and** `.model = Some(<Node object export>)`, reseal and
read. The `wrong_export` case in the suite performs exactly this model
substitution with a null type and is refused `Invalid::Binding`; with the type
field populated the same substitution is accepted. That is the attempt record's
own model standing in as the effect's authority — the substitution
`docs/compiled-protocol-v1.md:419` and `FR-042` line 146 exist to forbid. The
compiler never emits a typed compensation effect today (the grammar has no such
view), so this is only reachable through a forged or resealed package, which is
precisely the reader's threat model and the one the adverse tests use.

Fix: dispatch on `binding.kind == BindingKind::CompensationEffect` alone, and
inside `compensation_effect_identity` require the operation model in both lanes,
requiring the declared value type as well when it is present. Derived from the
code; not executed here, since a reproduction needs a new test and this review
makes no test edits.

### FND-002 — half the compensation chain is load-bearing

`compensation_bindings` (control.rs:727) genuinely pins four links: registration
→ (forward effect, owner role), activation → registration, attempt →
(activation, owner role), effect → attempt, each with matching compensation
subject, anchor kind/owner/binding and the registered ObservationBinding
contract. The adverse test removes one real prerequisite per case and gets
`Invalid::Binding`, so those four are demonstrably load-bearing and finite (each
`compensation_binding` call is one bounded scan of `package.definitions`, charged
through `Work`).

The recovery half is not. `compensations()` validates `value.clock` as
`&[BindingKind::Clock]` (control.rs:863) and every `recovery_bindings` entry as
one of six kinds (control.rs:911), with no subject, anchor, model or `requires`
constraint. The compiler emits snapshot → activation and progress/closure →
(clock, effect, snapshot), and the positive test asserts all of that on the
emitted side, but the reader accepts a package where `Full` lists `Partial`'s
snapshot, where the snapshot's `requires` no longer names the activation, or
where the two obligations' clock bindings are swapped. Controls have the
analogous check (`timeout_authority`, control.rs:641); compensations have no
`recovery_authority`. Mitigating: the population/closure members of
`recovery_bindings` are still validated globally by
`models/populations.rs:138-242`, including the set-equality rule against
binders and anchored values, so a *fabricated* population cannot enter this way —
only a mis-attributed one.

### FND-003 — the typed-view lane is asserted in the TC and untested

TC-121 step 6 ends "an explicitly selected typed effect view retains its
payload/model fields", and `docs/compiled-protocol-v1.md:426` repeats it. No test
covers it, positively or negatively, and FND-001 is the reason that matters: the
untested lane is the one that is unenforced. A negative test is available today
without any grammar change — reseal with a type added and a foreign model, expect
refusal.

### What the tests actually establish

- Seven tests, all passing (`/tmp/quire-native-compensation-await-test.log`).
  Every positive case runs actual source through `with_proofs` → `discharged` →
  `native::admit` → `native::emit` → an independent `read`, and asserts
  `read.package() == package` and digest equality. No wire fixture supplies the
  oracle; expectations are read from the original `ComposedUnit` AST, `scope`
  and `typed` reports.
- The main positive test carries the non-obvious ones: two obligations sharing
  one `forward_effect` (line 192), a temporal requirement interleaved so the
  authored requirement ordinals are `[0, 2]` while the compact compensation
  handles are 0 and 1 (line 205), registration/activation/attempt/effect
  instance indices deduped across both obligations to prove distinct identities
  (line 338), `commit Main::Committed` versus an explicit `never` (line 341), and
  the two `sum<M::Total>` recovery queries resolving to the `Total` nominal type
  (line 349). Capture binders are matched back to the original
  `scope.binders`/`typed.binders()` anchors, not just to the wire.
- The seventh test is not a duplicate of the first: `await ... after Partial`
  emits `AwaitAnchor::Compensation`, the associated `event ... for Partial`
  carries `compensation: Some(handle)`, the await clock requires the
  compensation's activation binding, progress/closure require that clock, and
  five causal edges (2 × `AwaitSuccess`, `AwaitTimeout`, 2 × `Join`) are asserted
  by kind, endpoint and port with `maximum` absent. Original loci and the
  authored `Partial` source slices are compared, so the ordinal→handle mapping is
  checked from the source side.
- Adverse coverage is real, not `is_err()`: expired/later binder scopes assert
  the exact `ScopeIssue::OutOfScope` name and source slice, wrong control kinds
  assert `WrongTargetKind` on the required `StructuralKind`, zero attempts and
  `[31,30]` assert `Invalid::NumericDomain`, `M::Plain` attempts assert
  `Invalid::Type`, and the unsafe retry case asserts an unproved `CheckedRange`
  obligation at the original `earlierFull.tally + 1` span. `changed()` asserts
  each mutation site is unique, so no adverse case silently rewrites nothing.
- Seam compliance: no `#[cfg(test)]` behaviour branches, no test-only feature
  flag, no forged admission report. The `serde_json` reseal in the identity test
  is a decoder-boundary probe of the public reader, labelled as such in the
  source comment. The synthetic producer/baseline is declared and is not claimed
  as the FR-042-AC-10 handoff.

### Rust idioms and panic surface

No `unwrap`/`expect`/`panic!`/slicing on any library path in the new code.
`compensations.rs` replaces the previous direct indexing with `get`/`get_mut` +
`ok_or` in `runtime.rs` (`initializers`, `bind_anchor`, the Finish closure
lookup), which is a net improvement carried by this change. Arithmetic is
`saturating_add` on name/byte charges, `parse::<i64>()` with an explicit
`NumericDomain` refusal and a `maximum <= 0` guard mirrored by the reader at
`validate.rs:143`; the retry relation's own arithmetic safety is a proof
obligation, exercised adversely. Every new loop is bounded by an already-admitted
table and charged before the work. `structural_symbol` returning `&Symbol`
instead of a handle keeps the compensation/control split at one call site rather
than duplicating the reference scan. `role_handle`'s widening to
`Option<ControlId>` is the right shape for a declaration-level role reference
(`site == None`) and does not loosen the existing control-site matches.

Two portable-style notes, filed as low because the repo documents no idiom to
test against: `lower_one` is a single ~300-line function that would read better
split at the retry boundary (it matches the length of its neighbours in
`runtime.rs`, so this is consistent with the file, not with `rust-style`), and
`quire coverage` raises `oracle-resembles-implementation` on the `integer`
helper at `tests/native_compensation_emission.rs:149` (token similarity 0.86
against `integer_value` in `tests/native_protocol_emission.rs`). The helper only
unwraps a checked wire number for comparison against authored literals `(0, 30,
3)` and `(0, 5)`, so it is not a re-implemented oracle; the same suspicion
already stands against the older file.

### Gates inspected

Read from the root's completed logs; green heavy gates were not rerun.
`/tmp/quire-native-compensation-integrated-{fmt,focused,clippy-minimal,clippy-all,test-minimal,test-all,spec}.log`:
`cargo fmt` empty; both strict all-target Clippy configurations
(`--no-default-features` and `--all-features`, `--locked`, `-D warnings`)
`Finished` with no warnings; full suites `553` minimal and `569` all-feature
passing across 56 `test result` lines each (five doctests included), 0 failures,
4 inherited ignores (3 × `fixture_audit` IT-004, 1 × `native_backend` LC04);
focused run 6 compensation + 10 query-proof + 4 native-query, all passing.
The seventh compensation test post-dates those full runs and is covered by
`/tmp/quire-native-compensation-await-{test,clippy,fmt}.log`: 7 compensation
tests passing, strict all-target minimal Clippy `Finished`, `fmt` empty. No
`deny.toml` in this repo, so no `cargo deny` lane. `quire coverage --scope
/home/peter/dev/worktrees/quire-language-native-compensation --json` re-run here:
367/376 backed, 0 status lies, unchanged from the previous increment.
