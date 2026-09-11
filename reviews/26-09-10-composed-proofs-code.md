---
id: SR-327
title: "Code review of composed guarded-definedness proofs"
type: SpecReview
analysis: code-review
scope: "FR-040 / TC-119; src/checking/composed/proofs.rs; src/checking/composed/proofs/{correspondence,dependencies,work}.rs; src/checking/composed/proofs/engine.rs; src/checking/composed/proofs/engine/{graph,roots,walk}.rs; src/checking/composed/sources.rs; src/checking/proof.rs; src/checking/proof/{facts,graph}.rs; tests/composed_proofs.rs; README.md; spec/{spec.md,model-linking/tests.md,test-cases/TC-119-check-composed-values.md}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-119
    type: references
---

## Summary

Reviewed `agent-a/composed-definedness` (0dab1ce) against
`agent-a/composed-type-admission` under `code-review` + `rust-review` +
`rust-style` and the repo's `AGENTS.md`/`CLAUDE.md` idioms, plus a narrow
recheck of the inherited SR-325 corrections in f768d62. The stage does what it
claims: one shared materializer serves both provers rather than a second
kernel, charge-before-work precedes every allocation and retention, guard
provenance is carried by binder identity rather than by expression spelling, and
unsupported query/representation meanings stay explicit instead of becoming
discharge. Findings are a misleading completeness accessor, a representation
abstraction that leaks into exported IR, and panic-on-invariant surface — no
false discharge was found.

## Verdict

**CONDITIONAL** — no high finding; four medium and two low. All local gates are
green at this HEAD. The delivered definedness stage is accepted as delivered;
full FR-040, family/runtime admission and #40 source-to-B emission remain open
(see SR-328).

## Gates

Executed serially at 0dab1ce in one batch under `flock
/tmp/quire-heavy-check.lock`, `src/lib.rs` mtime touched once before the first
gate, each `CARGO_BUILD_JOBS=1
CARGO_TARGET_DIR=/tmp/formalization-a-language-target nice -n10 cargo ...
--locked`, tests `-- --test-threads=1`. Logs
`/tmp/quire-composed-proofs-review-{clippy-minimal,clippy-all,test-minimal,test-all,fmt}.log`.

| Gate | Result |
| --- | --- |
| `clippy --all-targets --no-default-features -- -D warnings` | exit 0 |
| `clippy --all-targets --all-features -- -D warnings` | exit 0 |
| `test --all-targets --no-default-features` | exit 0 — 473 passed, 0 failed, 4 ignored |
| `test --all-targets --all-features` | exit 0 — 489 passed, 0 failed, 4 ignored |
| `fmt --check` | exit 0 |
| `cargo deny check` | skipped — no `deny.toml` in the repository |

`--all-targets` excludes doc-tests, so these totals sit two below the root's
475/491 for the same tree; the two `compile_fail` doc-tests in `src/package.rs`
are the difference, not a regression. Inside those runs: `composed_proofs` 14
passed, `composed_types` 21 passed, historical `checking` 24 passed.

## Findings

| ID      | Severity | Summary                                                                                                      | Refs                                                          |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| FND-001 | medium   | `DeclarationProof::complete()` returns true for a declaration refused by an unproved goal                      | src/checking/composed/proofs/engine.rs:128, src/checking/composed/proofs.rs:136 |
| FND-002 | medium   | Text scalars, enums, records, objects and references are exported into the IR environment as `Boolean` inputs  | src/checking/proof.rs:217, src/checking/proof.rs:238, src/checking/composed/proofs/engine.rs:422 |
| FND-003 | medium   | The historical `/1` proof type map became partial: two previously total scalar paths now `unreachable!`        | src/checking/proof.rs:209, src/checking/proof.rs:227           |
| FND-004 | medium   | Cross-module invariants asserted with `expect`/`unreachable!` instead of the stage's own typed causes          | src/checking/composed/proofs/engine/walk.rs:49, src/checking/composed/proofs/correspondence.rs:43 |
| FND-005 | low      | `roots` re-derives the declaration arena range by hand instead of the shared `owned_range` helper              | src/checking/composed/proofs/engine/roots.rs:24                |
| FND-006 | low      | Per-node `Kind` clone in the shared materializer and a `String` key allocation on every field lookup           | src/checking/proof/graph.rs:36, src/checking/composed/proofs/engine.rs:151 |

## Detail

**FND-001 (CONFIRMED by construction; not executed).** `run()` records an
`Unproved` cause and continues the pending-goal loop (`engine.rs:571-578`), so
it returns `Ok(())`; the caller then sets `local_complete = true` and
`complete = entry.references().is_empty()` (`engine.rs:127-129`) without
consulting `output.causes`. Failure scenario: the leaf predicate `amount + 1 <=
10` with no preceding guard — exactly the negative case at
`tests/composed_proofs.rs:229-247` — has one `Unproved` cause and no
dependencies, so `disposition()` is `Refused` while `complete()` is `true`,
contradicting its doc comment ("Local discharge and all required native
dependencies have completed"). `dependencies::finish` can also set `complete` on
a caller whose own goal failed (`dependencies.rs:83-85`). Nothing in the stage
reads `complete` except `disposition()`, which checks causes first, so no
current result is wrong — but it is a public accessor a downstream emitter would
reasonably gate on, and no test pins it (`unproved()` asserts only the
disposition, `tests/composed_proofs.rs:117-120`). Gate it on
`self.causes.is_empty()`.

**FND-002.** `representation(.., ComposedValues)` maps a `Text` scalar to
`ir::ValueType::Boolean` (`proof.rs:216-221`) and every `Enumeration`, `Record`,
`Object` and `Reference` to `Boolean` (`proof.rs:234-238`). `symbolic()` turns
that into a real `ir::ValueDeclaration` pushed into the public
`DeclarationEnvironment` (`engine.rs:422-427`, `engine.rs:534-546`), so
`DeclarationProof::environment()` describes an authored text field or record
receiver as a two-valued input. Within this stage no operation is emitted over
those symbols — comparisons on non-numeric operands become a fresh symbolic
Boolean (`walk.rs:311-316`) rather than an `ir::Compare` — so I found no
discharge that depends on the abstraction. The exposure is downstream:
`Unsupported::ValueRepresentation` never fires for these types, so a consumer
(#40 source-to-B emission) cannot distinguish "represented" from "abstracted",
and the pinned finite-domain checker would see a 2-element domain for an
unbounded text value the moment any equality or record operation is lowered.
The brief's rule is explicit — no symbolic Boolean abstraction of
records/text/enums in exported IR — so this should be either a distinct
`ValueType` or an `Unsupported` cause, not a silent `Boolean`.

**FND-003.** Before this branch `proof_type` was total over `NativeType`: any
scalar that was not integer-represented fell to the `Boolean` arm
(`agent-a/composed-type-admission:src/checking/proof.rs:173-190`). The shared
rewrite now aborts the process for a rational-represented scalar under
`HistoricalFinite` (`proof.rs:209`) and for any scalar whose representation is
not a primitive (`proof.rs:227`). Both are guarded by real invariants —
historical `Catalog::formal` returns `None` for rational (`types.rs:370-373`)
and model admission refuses a role that disagrees with its site representation
(`native_model/admission.rs:400-431`) — so no admissible input reaches them
today and the historical fixtures pass unchanged (24 checking tests, gate
above). It is still a behavioural change on the `/1` path the brief asks to
leave untouched: a panic replaced a value. A `Diagnostic` return costs nothing
here, since the function already returns `Result`.

**FND-004.** The new stage adds ~35 `expect`/`unreachable!` sites. Most are
closed-enum arms and read as intended. Two are load-bearing across module
boundaries. `read()` does `typed.node(at).expect("original typed occurrence")`
and `node.binder.expect("bound direct read")` (`walk.rs:48-49`) — a name
occurrence that scope binding left unresolved but typing admitted downs the
caller instead of producing a cause, and the same shape recurs at
`walk.rs:107,168,440,445` and `engine/graph.rs:46`. Worse for maintenance:
`correspondence.rs:41-46` translates the source stage's work dimensions and
`unreachable!`s on five of them. That module does not own
`Correspondence::collect`; the day `sources.rs` charges `Declarations` there,
the failure is a runtime abort in a different file rather than a compile error.
Map it to a conservative dimension. This is the same class as SR-325 FND-002,
whose disposition was a typed refusal, and `CauseKind` already has the variants.

**FND-005.** `roots()` recovers the declaration's arena window from
`typed.nodes().first()` and indexes `child[id.0 - start]` (`roots.rs:11,24`).
f768d62 extracted exactly this arithmetic into
`syntax::composed::arena::owned_range` with its contiguity invariant documented
and exercised through both existing callers (`solver.rs:141`,
`linking/composed/arena.rs:21`); the new stage reimplements it without the
helper, the invariant comment or an assertion. If a declaration's typed nodes
were ever non-contiguous or partially typed, the subtraction wraps in release
and panics as an opaque out-of-bounds. Reuse the helper, or assert
`nodes.len()` against the owned range.

**FND-006.** `materialize` now clones each node's `Kind` per visit
(`proof/graph.rs:36`) where the historical version matched by reference; every
`Witness` clone copies an `ir::ValueType`. `Key::Field` stores an owned `String`
(`engine.rs:151`), so `key()` allocates the field name on every lookup including
cache hits (`walk.rs:139-147`). Both are bounded and charged, and the `String`
is forced by `Key<'a>` being tied to the model lifetime rather than the unit
borrow; noting them only so the next expansion of the graph does not inherit the
cost silently.

## Inherited correction recheck (0bc06d7..f768d62)

Narrow recheck only; the SR-325 findings were latent, not reproduced defects,
and are not reopened here.

| Prior finding | Disposition |
| --- | --- |
| SR-325 FND-001 | **Resolved.** The model-value owner scan skips unselected catalogs and a missing catalog yields `CauseKind::UpstreamBinding` rather than `expect`. |
| SR-325 FND-002 | **Resolved in the parent files.** Callee-kind, record and profile paths return typed refusals. Recurs in *new* code — see FND-004. |
| SR-325 FND-003 | **Resolved.** `syntax::composed::arena::owned_range` is shared by binding and typing, its invariant is documented and constructor-private, the range assertion is active in release, and the real-parser controls exercise all three arenas. Not adopted by the new `roots` — see FND-005. |
| SR-325 FND-004 | **Resolved.** Query/graph permissions are named enum choices; no silent profile-use-0 fallback remains. |
| SR-325 FND-005 | **Resolved.** Positional `bool` permissions and the `settled`/`count` naming are fixed. |

No regression found in the corrected files. The new public type control retains
the selected-model values beside an unselected input and refuses equal duplicate
supplies upstream, and the N10001 model-domain refusal is now tested; 21 focused
arena/type controls pass in the gate runs above.
