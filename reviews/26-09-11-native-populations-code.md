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

Review of `agent-a/native-population-exports` at `8590407` against its parent
`origin/main` `09dafe4` (which already carries the native emission review
SR-333/SR-334): 16 files, +1505/-45, adding `NativeType::Reference` and
`Population` exports, the runtime population/closure requirement derivation, the
`Reaches` lowering, and the reader-side population validator. Skills applied:
`agent-skills/code-review`, which dispatches the Rust lane to
`agent-skills/rust-review`, with `agent-skills/rust-style` as the portable idiom
default because this repository documents no Rust idiom skill of its own. The
emitted requirement set is correctly derived from the exact admitted role; the
one substantive finding is on the reader, which checks that set for sufficiency
but not for necessity.

## Verdict

**CONDITIONAL** — no high finding. The exact-role authority the change is for is
real on both sides: the export triple, the `Reaches` operands and the edge field
are pinned by pointer identity against the catalog's canonical `ObjectRole`, not
by label equality, and the five new tests carry that through parse, bind, type,
proof, emit and an independent read. The medium finding is a one-directional
check in the independent reader, not a defect in the emitted artifact.

## What the review verified

- **Exact-role selection, not matching labels.** `reference_target`
  (`models.rs:679`) resolves all three exports and requires one model index and
  `std::ptr::eq` between the reference, object and population targets;
  `Catalog::composed` builds `objects`/`references` from one
  `model.roles().objects` allocation, so pointer identity is the catalog's
  canonical role and the comparison is meaningful rather than incidental.
  `Validation::value` repeats it for the `Reaches` universe and re-derives the
  edge role through `catalog.formal(field.value_type(), ..)`, so a field whose
  declared leaf is a reference into a different universe refuses
  (`models/populations.rs:120-134`).
- **Universe labels are not identities.** Object roles are unique by `record`
  and by `reference` at admission (`native_model/admission.rs:461`), `universe`
  is not; both export tables key on `(kind, record, universe)` with the role's
  own `source` locus, so two roles sharing `nodes` produce two distinct
  population exports. `original_object_roles_grant_distinct_populations_...`
  fails if either is collapsed.
- **Operand origin, type and result.** The composed solver unifies the `Reaches`
  operands (`solver/expressions.rs:161`) and requires a Boolean root; the reader
  independently re-checks `start.value_type == end.value_type`,
  `start.origin == end.origin` and `Type::Boolean`
  (`models/populations.rs:71-76`), so the emitter's origin-only check is not the
  only gate.
- **Anchors are preserved, not collapsed.** The collector skips
  `Let`/`Capture`/`Query` binders so a derived token cannot mint a population at
  a later anchor, refuses a `Selected` origin from a foreign unit, and keys needs
  on the observation anchor index; the reader's `required` walk applies the same
  binder-kind and `Origin::Anchor` rules. `captured_pre_references_keep_...`
  pins the captured `pre` read whose origin anchor is `InvocationPre` while its
  evaluation anchor is `InvocationPost`, and asserts both population anchors on
  the post-declaration.
- **Closure never asserts truth.** Every population gets exactly one `Closure`
  requiring exactly it, with the same value type, model, anchor and scope
  (`models/populations.rs:216-232`), selecting `ObservationBinding` and
  `Progress` and verifying the named definition against the binding's contract
  dependency and authority (`contract`, `:236`). Membership and completeness are
  never computed; `validate.rs:676` keeps the requirement graph acyclic.
- **Termination and bounds.** Both record traversals push a work stack but
  charge `Dimension::Entries` before every push and every set insertion, and both
  carry a visited set keyed on `(model, record, anchor)`, so a cyclic record
  graph terminates and a wide one exhausts the budget instead of the heap.
  `Dimension::Depth` is a peak dimension (`work.rs:192`), so charging the running
  depth each iteration is the correct accounting, not an accumulation bug. The
  asymmetric `charge(Entries, 1)` before the closure `runtime.add` is also
  correct: it reserves the one-element `requires` vector that the population
  binding only allocates when its anchor has a prerequisite.
- **No composed producer contract was invented.** `ValueOperation::Parent` is
  now an explicit `Unsupported::Feature` with a comment stating that population
  identity does not establish its result semantics (`models.rs:260`), and
  relationship/component/endpoint exports still refuse in `export_key`.
- **Hygiene.** No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`,
  `dbg!`, `#[allow]`, `TODO` or `FIXME` in either new module; SPDX header and a
  `//!` requirement citation on both; `#![forbid(unsafe_code)]` intact.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The independent reader checks the population requirement set for sufficiency but never for necessity. `required` walks every seeded binder/value and refuses when a reachable object role has no `(model, record, anchor)` entry, but nothing iterates `pairs` to assert each entry was seeded. A package carrying an extra well-formed `Population`+`Closure` pair — any object role of a bound model, at any anchor of that declaration, with a fresh name and the anchor's own prerequisite — satisfies `bindings`, `required`, `validate.rs` name uniqueness and the acyclic requirement graph, and is admitted. The effect is an artifact that demands consumer `ObservationBinding`/`Progress` inputs no authored value justifies; the emitter derives the set correctly, so only a tampered or foreign-produced package reaches this. The one-axis tamper matrix covers substitution and removal but has no addition axis, so no test would fail. Assert the seeded set covers `pairs` exactly, and add an added-pair axis | src/protocol_artifact/models/populations.rs:257; src/protocol_artifact/models/populations.rs:304; src/protocol_artifact/models/populations.rs:216; tests/native_population_emission.rs:290 | missing-requirement |
| FND-002 | low | Two sites rely on the same unstated linking invariant with two different strengths, and neither cites it. The emitter keys population needs on `model.environment().owner()` alone, which is weaker than this crate's own model identity `same_model` (owner **and** digest, `models.rs:812`) and weaker than the reader's model index; the composed solver's graph rule admits an edge whose leaf reference is in another model with an equal owner (`checking/composed/solver/validation.rs:265`), while the reader requires pointer identity within one catalog. Both are safe today only because linking refuses two selected inputs that share an owner — different digests become `ModelConflictKind::Owner` and equal digests become `AmbiguousSelection` (`linking/composed/models.rs:354`, `:460`). If that ever relaxes, two models collapse into one `Need` and the emitter drops a requirement its own reader then rejects. Key on the model input index or on owner+digest, or state the invariant at the use site | src/protocol_artifact/native/populations.rs:27; src/protocol_artifact/native/populations.rs:147; src/protocol_artifact/native/populations.rs:166; src/checking/composed/solver/validation.rs:265 | missing-requirement |
| FND-003 | low | Style note, no failing scenario; three new sites in the class already recorded as SR-333 FND-003. `runtime.rs:602` indexes `runtime.anchors` with a `layout.anchor()` position — in range only because both vectors are built from one pass over `layout.anchors` (`runtime.rs:315`) — and `models/populations.rs:221-222` index `declaration.bindings` with values captured from `enumerate()` over that same slice. The module's own idiom elsewhere is `.get().ok_or(..)`; the inconsistency is that an edit to either side of one of these pairings converts a typed error into a panic | src/protocol_artifact/native/runtime.rs:602; src/protocol_artifact/models/populations.rs:221; src/protocol_artifact/models/populations.rs:222 | missing-requirement |
| FND-004 | low | Evidence granularity. `foreign_object_graph_edges_refuse_before_native_family_admission` asserts `TypeMismatch` and `Unsupported::FamilyProof`, which is the refusal of the whole declaration, not specifically of the crossed graph edge; `CauseKind::InvalidGraphEdge` — the cause the new `Reaches` path actually depends on — is asserted nowhere in the suite. A same-role edge whose field is not a reference, and a `reaches` whose edge field belongs to another record, would pin that cause directly | tests/native_population_emission.rs:470; src/checking/composed/solver/validation.rs:254; src/checking/composed/solver/validation.rs:267 | correct-requirement-no-evidence |

## Gates

Root-supplied local logs for this exact source; inspected, not re-run. No gate
was re-run: FND-001 is settled by reading the two validators, and the rest are
style or evidence notes that source inspection settles.

- `cargo fmt --all -- --check` — `/tmp/quire-native-populations-fmt.log`, empty.
- `cargo clippy --locked --all-targets --no-default-features -- -D warnings` —
  `-clippy-minimal.log`, no diagnostics.
- Same with `--all-features` — `-clippy-all.log`, no diagnostics.
- `cargo test --locked --no-default-features -- --test-threads=1` —
  `-test-minimal.log`, 52 suites, 0 failed.
- Same with `--all-features` — `-test-all.log`, 52 suites, 0 failed;
  `native_population_emission` 5/5, `native_protocol_emission` 15/15,
  `protocol_artifact` 24/24, 5 doctests including the `native::emit`
  `compile_fail` case.
- `quire validate --scope <worktree> --summary 'spec/**/*.md'` — `-spec.log`,
  398/398 docs grammar-clean, 0 grammar findings.
- The earlier `-merged-focused.log` (5 population, 15 native, 24 reader, 9
  numeric) precedes only the `BTreeMap` `Entry` idiom correction in `9814402`,
  which the full logs above supersede.
- No `deny.toml` exists, so `cargo deny` does not apply.
