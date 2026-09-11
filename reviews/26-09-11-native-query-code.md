---
id: SR-343
title: "Code review — ordered query definedness and native query emission"
type: SpecReview
analysis: code-review
scope: "src/checking/composed/proofs.rs; src/checking/composed/proofs/engine.rs; src/checking/composed/proofs/engine/queries.rs; src/checking/composed/proofs/engine/walk.rs; src/protocol_artifact/native/layout.rs; src/protocol_artifact/native/values.rs; tests/composed_query_proofs.rs; tests/composed_proofs.rs; tests/native_query_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-119
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

`/code-review` with the Rust lane from `/home/peter/dev/agent-skills/rust-review/SKILL.md`
and the portable `rust-style` defaults (the repo declares no Rust idiom doc of its
own), over `agent-a/native-query-proofs` diffed against
`agent-a/native-population-exports` at `667226b`. All eight ordered query forms
are implemented in the real composed definedness path and lowered from their
original AST occurrences; the query proof and emission tests are behavioural and
non-tautological. No `high` finding; the substantive one is a work ceiling that
makes `sum` undischargeable at the declared maximum capacity.

## Verdict

**CONDITIONAL** — implementation and tests are real and source-owned; one medium
capacity ceiling and one medium dead-API finding, plus three low items.

## Findings

| ID      | Severity | Summary                                                                                       | Refs                                                        | Escape Cause                        |
| ------- | -------- | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ----------------------------------- |
| FND-001 | medium   | `sum` proof cost is linear in declared capacity; no sum over a 10,000-element domain can discharge | src/checking/composed/proofs/engine/queries.rs:274; src/checking/composed/proofs/work.rs:60 | correct-requirement-no-evidence     |
| FND-002 | medium   | `Unsupported::OrderedQuery` is now unconstructible public API with no producer and no test      | src/checking/composed/proofs.rs:32                            | implementation-bug-despite-evidence |
| FND-003 | low      | Emitted binder scope locus spans the collection, so locus containment is not an availability oracle | src/protocol_artifact/native/layout.rs:462; docs/compiled-protocol-v1.md:533 | wrong-requirement                   |
| FND-004 | low      | Rational sum gate requires denominator 1 on both endpoints; the mixed 1/2 case has no test      | src/checking/composed/proofs/engine/queries.rs:288; tests/composed_query_proofs.rs:535 | correct-requirement-no-evidence     |
| FND-005 | low      | `query()` re-destructures a node `walk.rs` already matched, leaving an unreachable arm; `element()` returns a bare 3-tuple beside a named `Body` | src/checking/composed/proofs/engine/queries.rs:191; src/checking/composed/proofs/engine/queries.rs:46 | missing-requirement                 |

### FND-001 — sum cannot reach the declared maximum

`sum_prefixes` emits two `goal()` calls per prefix `k <= N`, each materialising a
`Witness` plus the path implication chain, and three graph nodes per endpoint
transition. `ProofLimits::bounded()` clamps every caller-supplied dimension
downwards against `Limits::default()`, so `goals` can never exceed 10,000 and
`graph_nodes` never exceeds 100,000. A sum whose endpoint arithmetic stays in the
Total domain therefore runs all `N` prefixes and needs `2N` goals: any `N > 5,000`
exhausts `Dimension::Goals` and returns `ProofDisposition::Unfinished` rather than
a semantic verdict, and no caller can raise the ceiling.

Concrete scenario: `LargeTotal 0..200000/U`, `Amount 1..20/U`, sequence maximum
10,000, `sum<M::LargeTotal>(item in input.amounts: item) >= 0`. The arithmetic is
in range at every prefix (upper endpoint 200,000), so the early `break` never
fires; the run terminates in Goals exhaustion. This is an honest bounded refusal,
not a wrong answer, but FR-040-AC-5 names N=10,000 as a checkable maximum, and
`empty_results_and_maximal_declared_domains_keep_their_actual_admission_boundaries`
exercises 10,000 with `size`, `count` and `forall` only — all of which are O(1)
in `N`. `sum` is the only form whose proof work scales with declared capacity and
it is the one form not covered at that bound. Derived from the code and the
clamped limits; not executed here, since establishing it needs a new test and this
review makes no test edits.

### FND-002 — dead refusal variant

`Unsupported::OrderedQuery` has no remaining construction site anywhere in `src/`
or `tests/` (the three former sites in `walk.rs` now dispatch to `size`,
`quantified`, `contains` and `query`, and `composed_proofs.rs` dropped its
assertion). It stays `pub` in the public `Unsupported` enum, so consumers carry a
match arm for a refusal the engine cannot emit — rust-review §3 "public API with
no caller and no test". Either remove it in the same breaking change as the other
query work or document it as reserved.

### FND-003 — scope locus is wider than availability

The lexical region pushed for a query binder is `[binder_name.start, body.end]`,
which lexically covers the collection expression sitting between them. The value
handles are correct — `native_query_emission.rs:263` asserts
`domain_value.scope == value.scope` and the reader's containment check at
`validate.rs:766` is satisfied by the parent chain — but `docs/compiled-protocol-v1.md`
now states "a binder is available in its body, not its collection" without saying
that the per-value `scope` handle, not `scopes[].locus`, is the authority. A
consumer inferring availability from locus containment would read `kept` as
visible inside `filter(...)` in `forall(kept in filter(...): ...)`. Pre-existing
for `let`; the query lowering extends it to a form the doc now makes a normative
claim about.

### What the tests actually establish

- `tests/composed_query_proofs.rs` (9 cases, all passing): `IndependentElements`
  is the load-bearing independence control — two binders over the same filtered
  collection, where `lhs != 0` must not discharge the divisor for `rhs`; it is
  asserted `Refused` with `NonZeroDivisor` at the original `rational(0,1) / rhs`
  span. `Division` is its positive twin (same binder, same occurrence, discharges).
  `Guarded` third conjunct proves a filter predicate guards the consumer body;
  `PartialFilter` proves an unguarded filter body still refuses. The
  `Intermediate` case is non-vacuous: with `Signed -10..10` and maximum 3, prefix 1
  passes and prefix 2 fails, and `accumulated - accumulated = 0` cannot repair it.
  `N=5` versus `N=6` over `Amount 1..20/U` into `Total 0..100/U` sits exactly on
  the boundary (prefix 5 upper endpoint = 100).
- `tests/native_query_emission.rs` (3 cases): `original_queries` builds every
  expectation from the original `ComposedUnit` AST — operator, binder span, domain
  and body `original_expression` indices, per-binder read handles, result type —
  and returns an occurrence histogram asserted as `[1,3,1,1,1,1,1,1]` and
  `[0,0,3,1,1,0,0,0]`. No wire table supplies the oracle. `native::admit` consumes
  an actual `ProofReport`; the refused and Goals-exhausted variants both return
  `Unsupported::FamilyProof`, so neither a failed nor an unfinished proof creates
  emission authority.
- Seam compliance: no `#[cfg(test)]` behaviour branches, no test-only feature
  flags, no forged private admission report, no wire fixture standing in for the
  parser. The synthetic baseline/producer in the library fixtures is not a real B
  handoff and is not claimed as one.

### Rust idioms and panic surface

`queries.rs` carries no `unwrap`/`expect`/`panic!`/slicing on a library path.
Accumulator arithmetic widens to `i128` before the domain test and narrows through
`i64::try_from`; `next_context` uses `checked_add`. Recursion through nested
sequence templates is bounded by the `Depth` high-water limit of 64, and the
`sequences` map is a DAG by construction (each entry keys a freshly minted
`ValueKey` and refers only to values built earlier). Charge-before-work holds in
every new path, including the clone of `Sequence.locals` and `path.guards` in
`element()` and `merged_path()`. `locals` and `context` are restored through a
closure so an error mid-body cannot leak a reinstantiated environment. The
`found.replace(index).is_some()` duplicate detector in `layout.rs:455` is correct
under `&&` short-circuiting but reads as a side effect inside a condition.

FND-005 is filed as `missing-requirement` because this repo publishes no Rust
idiom document of its own (rust-review §0), so these are portable `rust-style`
defaults with nothing in-repo to test against — not defects that escaped a gate.

### Gates inspected

`cargo fmt --all -- --check` (empty), `cargo clippy --locked --all-targets
--no-default-features -- -D warnings` and `--all-features` (both `Finished`, no
warnings), `cargo test --locked --no-default-features` and `--all-features`
(54 suites, 0 failures, 3 pre-existing `fixture_audit` ignores, 1 pre-existing
`native_backend` ignore, 5 doctests), focused run at
composed_query_proofs 9 / composed_proofs 16 / native_query_emission 3 /
native_population_emission 5 / native_protocol_emission 15 / protocol_artifact 24 /
protocol_number 9. Read from the root's completed logs at
`/tmp/quire-native-query-final-{fmt,focused,clippy-minimal,clippy-all,test-minimal,test-all,spec}.log`;
green heavy checks were not rerun. No `cargo deny`/`deny.toml` in this repo.
