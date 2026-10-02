---
id: SR-325
title: "Code review of composed type admission"
type: SpecReview
analysis: code-review
scope: "FR-040 / TC-119; src/checking/composed.rs; src/checking/composed/{sources,work}.rs; src/checking/composed/solver.rs; src/checking/composed/solver/{dependencies,expressions,literals,models,origins,roots,validation}.rs; src/checking/{types,variables}.rs; src/linking/composed/models.rs; tests/composed_types.rs; tests/composed_type_pipeline.rs; tests/support/composed_types/mod.rs; README.md; spec/{model-linking/tests.md,test-cases/TC-119-check-composed-values.md}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-119
    type: references
---

## Summary

Reviewed `agent-a/composed-type-admission` (621205d) against
`agent-a/composed-value-checking` under `code-review` + `rust-review` +
`rust-style` and the repo's own `AGENTS.md`/`CLAUDE.md` idioms. The stage is
honest work: charge-before-work is real in every dimension, `TypeDisposition`
keeps `Refused`/`Unfinished`/`Typed` separate and never manufactures success,
matches over the value/control/type enums are closed with no catch-all `_` arm,
one `Catalog`/`Variables` kernel is reused rather than duplicated, and the
obligations are recorded rather than discharged. The findings are a panic
surface reachable through caller-supplied inputs and one undocumented arena
invariant, not a design defect.

## Verdict

**CONDITIONAL** — no high finding. All five findings are medium or low. Every
local gate below is green at this HEAD; the delivered type-admission stage is
accepted as delivered, and FR-040/TC-119 remain open as a whole (see SR-326).

## Gates

Executed sequentially at 621205d under `flock /tmp/quire-heavy-check.lock`,
each `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/tmp/formalization-a-language-target
nice -n10 cargo ... --locked`, tests `-- --test-threads=1`. Log:
`/tmp/quire-composed-types-review-gates.log`.

| Gate | Result |
| --- | --- |
| `clippy --all-targets --no-default-features -- -D warnings` | exit 0 |
| `clippy --all-targets --all-features -- -D warnings` | exit 0 |
| `test --no-default-features` | exit 0 — 461 passed, 0 failed, 4 ignored, 47 suites |
| `test --all-features` | exit 0 — 477 passed, 0 failed, 4 ignored, 47 suites |
| `fmt --all -- --check` | exit 0 |
| `cargo deny check` | skipped — no `deny.toml` in the repository |

New controls inside those runs: `composed_types` 16 passed, `composed_type_pipeline`
2 passed.

## Findings

| ID      | Severity | Summary                                                                                                          | Refs                                                    |
| ------- | -------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| FND-001 | medium   | `catalog_at(..).expect("admitted catalog")` panics for a supplied model input that no unit import selected        | src/checking/composed/solver.rs:417                      |
| FND-002 | medium   | Cross-module invariants are asserted with `unreachable!`/`expect` in library code instead of a typed cause        | src/checking/composed/solver.rs:369, src/checking/composed/solver/expressions.rs:192, src/checking/composed/solver/expressions.rs:437 |
| FND-003 | medium   | `Solver::var` guards its subtraction with only `debug_assert!`, over an arena invariant nothing states or tests   | src/checking/composed/solver.rs:277, src/checking/composed/solver.rs:134, src/parser/composed.rs:98 |
| FND-004 | low      | An unmatched protocol `check` profile alias silently falls back to profile use 0 with no cause                    | src/checking/composed/solver/roots.rs:101               |
| FND-005 | low      | Positional `bool` permission API and two small clarity defects in the solver                                     | src/checking/composed.rs:318, src/checking/composed/solver.rs:231, src/checking/composed/solver/expressions.rs:321 |

## Detail

**FND-001 (PLAUSIBLE, not reproduced).** Since c92bf5a, `ModelBindings` builds
an `Exports`/`Catalog` only for inputs an import actually selects
(`models.rs:331`, `models.rs:426-432`); every other supplied input keeps
`catalogs[i] == None`. The new `catalog_at` (`models.rs:227`) returns `Option`
and documents that callers must enforce the owning declaration's disposition —
but the `BinderType::ModelValue` path selects its input by scanning *all*
inputs for `model.environment().owner() == &location.identity.owner`
(`solver.rs:413-418`) rather than by the unit's import selection, then
`.expect("admitted catalog")`s the result. Failure scenario: a caller supplies
two native models whose owners compare equal but only one of which any unit
imports; the scan returns the unimported index first and `admit_types` aborts
the process instead of returning the `CauseKind::UpstreamBinding` this same
function already falls through to at `solver.rs:433`. I could not build a
source-level reproduction under this review's read-only constraint — the
owner-conflict and ambiguous-selection refusals at `models.rs:355-399` and
`models.rs:501-524` appear to close the duplicate-owner door today — so this is
reported as a latent panic, not a live defect. The fix is a two-line
`if let Some(catalog) = ...` with the existing cause on the `else`.

**FND-002.** `Solver::catalog` resolves a `NativeType`'s owning catalog by
`std::ptr::eq` over `models.inputs()` and ends in
`unreachable!("bound native type belongs to input catalog")`. Every field
access, graph edge and operation context reaches it (`expressions.rs:433`,
`validation.rs:259`). Pointer identity is being used as a totality proof for an
invariant this module does not own: any future binder that hands back a
`NativeType` borrowed from a clone rather than from `inputs` converts a typed
refusal into a process abort. `expressions.rs:192`
(`unreachable!("namespace resolved predicate kind")` on a `PredicateCall` whose
target is not a `Predicate`) and `expressions.rs:437`
(`.expect("admitted record")` on a catalog record named by an admitted object
role) are the same shape. `rust-review` §6: on a library path a bad upstream
input should refuse, not down the caller. AGENTS.md's "preserve explicit
unsupported/incomplete results" points the same way, and `CauseKind` already
has `UpstreamBinding` for exactly this.

**FND-003.** `var()` computes `at.0 - self.range.start` behind a
`debug_assert!(self.range.contains(&at.0))`. Release profiles disable
overflow checks, so an out-of-range `ExprId` wraps to a near-`usize::MAX` index
and the following `self.profiles[..]` / `self.output.nodes[..]` panics with an
opaque out-of-bounds rather than the assert's message. The range comes from
`owned()`, a binary search over `span(&nodes[middle]).start`. That array is
**not** sorted by `span.start`: `add_value` (`src/parser/composed.rs:98`) pushes
children before parents, so `a + b` yields starts `0, 4, 0`. The search is
correct only because each declaration's expressions are contiguous and every
span nests inside the declaration span, making the `start < at` predicate
block-wise monotone. That precondition is the load-bearing invariant of the
whole stage and it is stated nowhere — not in `owned()`'s (absent) doc comment,
not in the `//!` header, and no test pins it. Failure scenario: a parser change
that emits a declaration's expressions non-contiguously (an interleaved
lookahead buffer, a shared subexpression cache) silently mis-slices the range
and turns a legal unit into a panic.

**FND-004.** In the protocol `check` handler the loop over
`definitions().declarations[..].uses` breaks on
`usage.alias.span == profile.span`; when nothing matches, `self.profiles`
stays at its `vec![0; ..]` default and the checked expression is admitted under
the declaration's *first* profile use, which may grant query or graph
permissions the authored `check` did not select. No `CauseKind` is recorded.
Binding refuses an unknown profile alias upstream, so this is a defence-in-depth
gap rather than a live escape, but every other unresolved selection in this
stage emits a typed cause and this one should too.

**FND-005.** `permissions()` returns a bare `(bool, bool)` consumed as
`let (q, g)`, and `require(at, queries, graph)` is called positionally at six
sites (`expressions.rs:122,159,166,280`, `roots.rs:9`, `validation.rs:210`);
`self.require(at, false, true)` does not read. A two-field struct or two named
methods removes the ambiguity. Two smaller items: `Solver::new` charges
`range.len() + scope.binders.len()` and then recomputes the identical
expression into `count` two lines later (`solver.rs:231-232`); and
`solve_relations`'s `let mut failed = vec![false; ..]` is bound as `done` in the
loop and means "settled", not "failed" (`expressions.rs:321-325`).

## Inherited correction recheck (ee865ec..c92bf5a, merged here)

Narrow recheck only; no previously settled finding is reopened.

| Prior finding | Disposition |
| --- | --- |
| SR-319 FND-018 | **Resolved.** `bind_models` now pushes `None` per native input (`models.rs:334`) and builds `Exports`/`Catalog` once, lazily, only when an import selects that input (`models.rs:426-432`), with the repeated-import cache lookup charged one `References`. The eager per-model indexing that consumed the 262 144 `Bindings` ceiling is gone; `multi_model_indexes_reserve_only_selected_inputs_and_share_repeated_imports` pins it, and the 16-model/eight-selected/nine-import vector passes in the gate runs above. |
| SR-319 FND-019 | **Resolved.** `charge_exports` now reserves entries per stored structure rather than approximately — 3 `Bindings` per type declaration, `fields().len()` twice for the shared map plus the ordered vector, a second `objects.len()`, and the previously free scalars index now charged one `References` each (`models.rs:891-921`). The borrowed operation parameters no longer charge entries they do not allocate. The contract doc at `models.rs:249-256` was rewritten to match the code, not the other way round. |
| SR-319 FND-020 | **Resolved.** `field()` is a charged binary search over the name-sorted `ordered_fields` with one `References` per comparison (`models.rs:812-829`), replacing the linear `find_charged` scan. `missing_field_binary_search_exhausts_before_its_last_comparison` pins the boundary. |
| SR-320 FND-011 | **Resolved.** The scale controls now cover `Bindings`, the dimension FND-018 exhausted, alongside the existing `References` controls (1 500 fields / 1 400 lookups). |

No regression found in the two changed files. The one new interaction is
FND-001 above: the lazy-catalog remedy is correct, but it makes `catalog_at`
legitimately `None` and this PR's new consumer `expect`s it. Charging a real
index entry or scalar read remains charged; no legal input below one budget is
being required to fit the independent budgets, and no hard ceiling moved.
