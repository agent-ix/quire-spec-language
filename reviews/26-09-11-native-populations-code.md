---
id: SR-336
title: "Code and Rust review of native population and reference exports"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/native/populations.rs; src/protocol_artifact/native/{types,values,runtime,context,metadata}.rs; src/protocol_artifact/models.rs; src/protocol_artifact/models/populations.rs; tests/native_population_emission.rs; tests/support/"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Recheck of this review's own findings at source commit `84aec59`, covering the
correction range `195ded9..84aec59` (7 files, +132/-26) on top of the increment
first reviewed at `8590407`. Skills applied unchanged:
`agent-skills/code-review`, dispatching the Rust lane to
`/home/peter/dev/agent-skills/rust-review/SKILL.md`, with `rust-style` as the
portable idiom default because this repository documents no Rust idiom skill of
its own. Three of the four initial findings are resolved in code and tests; one
is partially resolved and stays as a residual low, and two new low findings are
recorded. This is a targeted finding recheck, not a fresh repository campaign.

## Verdict

**CONDITIONAL** — no high finding, and the medium that carried the original
verdict is gone. The reader now checks both directions of its normalized
population traversal, so a valid-but-surplus `Population`+`Closure` pair refuses
`Invalid::Binding` at the surplus population binding's locus. What remains is
one citation residue and two evidence/style notes, none with a failing scenario
in the delivered artifact.

## Disposition of the initial findings

- **FND-001 (medium, necessity of the requirement set) — resolved.**
  `models/populations.rs:365-379` iterates `pairs` after the record traversal and
  refuses any key not in `seen`, setting `work.locus` to the offending population
  binding. The direction is exact rather than approximate: `pairs` keys are
  `(model, role.record, anchor)` and `Catalog::record_type` resolves a record
  name to `Object`/`Reference` before `Record` (`checking/types.rs:345-364`), so
  every record name reachable from a seeded binder or anchored value that carries
  an object role is *required* by the forward check at `:314` and *admitted* by
  the reverse check at `:368` under the same key. The `references` map is keyed on
  `role.reference` but resolves to the same `ObjectRole`, whose `.record` is what
  both sides insert, so object and reference uses of one role do not split the
  key. Evidence: the tamper matrix gained a `surplus_pair` axis
  (`tests/native_population_emission.rs:420-466`) that adds a well-formed Node
  population and its closure to `Right` — which is declared on `M::Other` — with
  a valid model, type, anchor, scope, contract authority and prerequisite, and a
  guard asserting `Right` carries no Node population beforehand, so the axis
  cannot pass vacuously. Node's type is already first-used in `Left`, so
  `validate.rs`'s canonical first-use type order is preserved and the vector
  reaches the population validator rather than failing earlier. Non-tautological:
  root ran this exact final vector against the pre-correction reader and it
  failed with `expected Invalid(Binding), admission succeeded` at
  `tests/native_population_emission.rs:472`
  (`/tmp/quire-native-populations-surplus-red.log`). Earlier intermediate red
  vectors had an ordering defect and are not treated as evidence.
- **FND-003 (low, raw indexing) — resolved at all three cited sites.**
  `native/runtime.rs:602` and `models/populations.rs:221-222` are now
  `.get(..).ok_or(Error::Invalid(Invalid::Binding))?` with a `work.visit()?`
  charged before each, matching the modules' own `.get().ok_or(..)` idiom. Three
  new charged steps, consistent with the repo's charge-before-work discipline;
  both full suites still pass, so the added accounting does not move any
  exercised limit.
- **FND-004 (low, evidence granularity) — resolved.**
  `foreign_object_graph_edges_refuse_before_native_family_admission` now drives
  two declarations and asserts the exact cause per declaration: `Crossed` →
  `CauseKind::TypeMismatch`, and a new `ScalarEdge` predicate whose `reaches`
  edge field is the scalar `n` → `CauseKind::InvalidGraphEdge`
  (`tests/native_population_emission.rs:526-556`). `InvalidGraphEdge` is now
  pinned independently of the crossed-universe mismatch, which is what the new
  `Reaches` path actually depends on.
- **FND-002 (low, model identity key) — partially resolved; retained below.**

## What the recheck verified in the correction range

- **Bounded work on the new path.** The reverse loop charges `work.visit()` and
  `work.bytes(record name)` per pair, and one further `visit` on the refusal
  path; `pairs` is already bounded by the binding count the forward pass charged,
  so the new check adds a linear, charged step and no new accumulator
  (rust-review §11).
- **Refusal cause and locus.** `Invalid::Binding` is the same cause the missing
  direction uses, so a surplus and a missing pair are indistinguishable to a
  consumer by cause and separated by locus — which is the behaviour FR-042 and
  the wire contract now state.
- **No new panic, unsafe or suppression surface** in the range: no `unwrap`,
  `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `#[allow]`, `TODO` or
  `FIXME` added to library code; `#![forbid(unsafe_code)]` intact; no lint,
  `[workspace.lints]` or threshold weakened (rust-review §5, §6).
- **Emitter/reader symmetry preserved.** The emitter still seeds from
  non-derived binders and `Anchored` values only, and the reader's `required`
  applies the same binder-kind and `Origin::Anchor` rules, so the new reverse
  check cannot refuse a faithfully emitted package. The round-trip equality
  assertion and all five population tests pass.
- **`Ok(())` fallthrough is still reachable and meaningful** — the reverse loop
  returns `Ok` only when every offered pair was reached, not by skipping.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | low | Partially resolved, retained. The emitter's owner-only `Key` now carries a comment citing the upstream linking invariant — same-owner/different-digest inputs refuse as `ModelConflictKind::Owner`, duplicate exact selections as `AmbiguousSelection` — and correctly does **not** claim that two distinct model owners are prohibited in one declaration; distinct owners remain supported and map to distinct wire model indices, so the owner-keyed emitter map and the index-keyed reader map stay bijective. The residue is the second site: `checking/composed/solver/validation.rs:265` admits a graph edge whose leaf reference lies in another model with an equal owner and still cites nothing, and `models.rs:812` `same_model` remains the stronger owner+digest key. If the linking guard relaxes, that site is the one that silently widens | src/protocol_artifact/native/populations.rs:27; src/protocol_artifact/native/populations.rs:150; src/checking/composed/solver/validation.rs:265; src/protocol_artifact/models.rs:812 | missing-requirement |
| FND-005 | low | New, evidence granularity on the correction itself. The reverse check deliberately sets `work.locus` to the surplus population binding's locus before refusing, and `Report::locus()` exposes it (`mod.rs:236`), but no axis of the tamper matrix asserts a locus — `failure()` compares only the `Error` variant. The locus attribution that distinguishes a surplus pair from a missing one is therefore unpinned: deleting the `work.locus` assignment leaves every test green. Assert `report.locus()` on at least the `surplus_pair` and `closure_pair` axes | src/protocol_artifact/models/populations.rs:376; src/protocol_artifact/mod.rs:236; tests/native_population_emission.rs:114; tests/native_population_emission.rs:470 | correct-requirement-no-evidence |
| FND-006 | low | New, style note with no failing scenario; residue of the class closed as FND-003. Two raw index sites remain in the touched module: `runtime.rs:151` indexes `meta.dependencies` with a value from `definition_dependency`, and `runtime.rs:447` indexes `meta.registered` with a profile index. Both are producer-side with internally derived indices and are not reachable from decoded input, so neither is a panic on an untrusted path; they are the last two sites in the module inconsistent with its `.get().ok_or(..)` idiom | src/protocol_artifact/native/runtime.rs:151; src/protocol_artifact/native/runtime.rs:447 | missing-requirement |

## Gates

Root-supplied serialized local gates for this exact correction source, inspected
rather than re-run; the worktree is clean at `84aec59` and the logs complete
between 02:40 and 02:45, immediately before the commit. No green heavy gate was
re-run for ceremony, and no new reproduction was needed: FND-002 is settled by
reading two call sites, FND-005 by reading the assertion helper, FND-006 by
inspection.

- `cargo fmt` — `/tmp/quire-native-populations-corrections-fmt.log`, empty.
- Clippy, all targets, `--no-default-features` and `--all-features` —
  `-corrections-clippy-minimal.log`, `-corrections-clippy-all.log`; both contain
  only `Checking`/`Finished`, no diagnostics.
- Focused — `-corrections-focused.log`: `native_population_emission` 5/5,
  `native_protocol_emission` 15/15, `protocol_artifact` 24/24,
  `protocol_number` 9/9.
- Full — `-corrections-test-minimal.log` 533 passed and
  `-corrections-test-all.log` 549 passed; 52 suites each including 5 doctests,
  0 failed, 4 inherited `#[ignore]`d.
- `quire validate ... spec` — `-corrections-spec.log`, 398/398 docs
  grammar-clean, 0 grammar findings.
- Red evidence — `/tmp/quire-native-populations-surplus-red.log`, the final
  surplus vector failing on the pre-correction reader.
- No `deny.toml` exists, so `cargo deny` does not apply.
