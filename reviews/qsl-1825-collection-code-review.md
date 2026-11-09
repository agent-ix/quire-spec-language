---
id: SR-61007
title: "Code and Rust review of QSL-1825 collection slice"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@dfe983639; git diff qsl-1825-handle...qsl-1825-collection: qsl-eval/src/value/collections.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/evaluate/invariants.rs, qsl-eval/src/execution.rs, qsl-eval/src/execution/scan.rs, qsl-jit/src/{kernel,exec,prepared,translate,coverage}.rs, qsl-conformance/src/generator.rs, qsl-jit/tests/it/{arena,collections,coverage}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-560
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: reviews
---
# Code and Rust review: QSL-1825 collection slice

## Summary

Ticket: QSL-1825. Branch `qsl-1825-collection` at dfe983639, four commits on
`qsl-1825-handle` (6ebae97b7). Scope is that diff only. Methods: code-review
with the rust-review lane (rust-style loaded). No PR exists for the branch.
gap-analysis was not run; the brief scoped this pass to code-review and
rust-review.

Examined: the old `evaluate.rs` iteration (`start_iteration`, `iterate`,
`finish`, `sum_step`, `iterate_seq`, Flatten, Size, Contains,
ConvertCollection) against the shared driver in `collections.rs` and the new
evaluator glue, line by line: charge points and their order, the visit
skipped for `only` over a `where` filter, the forall/exists early exit and its
retain after `note_stop`, the `only` population check before any read, the
exact-family sum seed (located at the body) and running total (unlocated),
the decimal loss record, the empty-sum zero for exact and non-exact families,
the count domain refusal, the fold and reduce accumulator, owner and trail
bookkeeping (`absorb_owner`, `set_slot_owner`, `note_element`, `note_stop`,
`note_computed` with `on_claim_level`), and every fault identifier including
the two fabricated-state orderings. The invariant probes in `invariants.rs`
only had field paths renamed; no expected identifier or charge changed. JIT:
loop emission, element and accumulator binding, the arena mark and
truncation, nested loops, body calls, `Halt::Family` and `StopAt` locations,
losses, the coverage and scan changes, `lists_type`, and the generator.
Tests ran green: `cargo test -p qsl-jit --test it -- collections arena`
(4 passed), `cargo test -p qsl-eval --lib` (208 passed, 1 failed:
`tc_454_published_index_sequence_is_the_samplers`, which needs `QSPEC_DIR` and
is unrelated to this diff), and `cargo test -p qsl-eval --test it --
collection refinement_populations named_population` (92 passed).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `coverage()` doc says `seq` is in the translated collection form, but scan names `seq` as its own form `seq`, which `FORMS` does not list, and translate does not handle it | qsl-jit/src/coverage.rs:51-58 |
| FND-002 | low | Every `Iteration` field is `pub`, and `Absorbed::Summand` expects the host to write the new total back into `accumulator`. A host that skips that write makes the next summand a fresh seed, which gives a wrong sum with no error | qsl-eval/src/value/collections.rs:326-354, qsl-eval/src/value/expression/evaluate.rs:4102, qsl-jit/src/kernel.rs:654 |
| FND-003 | low | `collections::sum_target` is public and has no caller | qsl-eval/src/value/collections.rs:646-653 |
| FND-004 | low | If the checker refuses a collection template, the whole generated unit is silently discarded, along with its base functions. No test asserts that any `c*` collection function is ever compared | qsl-conformance/src/generator.rs:650-672, qsl-conformance/src/generator.rs:768-782 |
| FND-005 | low | The new tests carry no `#[trace(...)]` binding, though `coverage.rs`, `clause.rs` and `differential.rs` in the same suite do. The collection rows of FR-560/FR-561 stay untraced in the matrix | qsl-jit/tests/it/collections.rs:227,250, qsl-jit/tests/it/arena.rs:68, qsl-jit/src/kernel.rs:753 |

## Verdict

The interpreter's behaviour is preserved. The shared driver reproduces the old
task machine's charges, stop points and fault identifiers:

- **Visits:** `charge_visit` runs in `next`, before the slot is bound, and is
  skipped only for `only` over a filter.
- **Reduce:** the first element's visit is charged in `start`, after the
  empty-reduction stop.
- **forall/exists:** the early exit still records `note_stop` and then
  retains the scalar.
- **Sum:** the exact-family seed check uses `admits`, which is what
  `sum_admits` reduced to for those families. The seed is located at the
  body and a later out-of-domain total is unlocated, as before. The decimal
  loss is recorded after the same `evaluate_decimal` call. Float and quantity
  sums stay on the evaluator's own path, and so does their empty-sum zero.
- **Map/filter:** `note_computed` gets the same sources and results through
  `keep = on_claim_level()`.
- **Fabricated states:** the spec-failure branch in `iterate` keeps the old
  order (pop, or visit charge, then `iteration-node-invalid`). It differs
  only for an exhausted, fabricated non-query node, where it now charges one
  visit before faulting. No checked program reaches that state.

The JIT loop is sound. `IterStart` records the arena length after the source
and identity handles, which stay below the mark. `IterAbsorb` copies the body
result into the driver before `IterNext` truncates. `IterFinish` truncates
before it pushes an owned result. Inner loops mark above the outer element,
so an inner truncation keeps the outer binder's handle. Callee Mark and
Reclaim nest inside one visit. A stop drops the per-call `State`. Each visit
charges a work unit, so the iteration count stays bounded by work units. The
JIT reports the `only` family result at the query node with no losses, as the
interpreter does, and uses the same population walk. The kernel's float, sum
and `forall`/`exists` refusals match `family_ok`. The arena peak test shows
the peak does not grow with the element count. All five findings are low.
