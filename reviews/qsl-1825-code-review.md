---
id: SR-61006
title: "Code and Rust review of QSL-1825 qsl-jit kernel value-handle path"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8e38d4860; git diff qsl-602-jit...qsl-1825-handle: qsl-jit/src/{kernel,exec,prepared,translate,coverage}.rs, qsl-eval/src/value/composite.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/evaluate/invariants.rs, qsl-eval/src/execution/scan.rs, qsl-conformance/src/generator.rs, qsl-jit/tests/it/*"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-562
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-568
    type: reviews
---
# Code and Rust review: QSL-1825 kernel value-handle path

## Summary

Ticket: QSL-1825. Branch `qsl-1825-handle` at 8e38d4860, three commits on
`qsl-602-jit` (55ec85cdd). Scope is that diff only. Methods: code-review with
the rust-review lane (rust-style loaded). No PR exists for the branch yet.

Examined: the arena and staging lifetime in `kernel::State` and `EntryFrame`,
handle validation (`handle`, `integer`, `rational`, `decimal`,
`take_staged`), representation tracking (`Repr`, `Carried`, `Ty`, `convert`,
if-join, call-argument boxing), charge, loss and outcome identity against the
interpreter's arms (Divide, Rational, Decimal, ConvertDecimal, order, Literal,
Field, Record, Tuple, stop paths), unsafe confinement (FR-568), the evaluator
refactor onto `qsl_eval::value::composite`, the rewritten source-shape test in
`invariants.rs`, the coverage and scan changes, the generator extension and
the new tests. Focused tests ran green: `cargo test -p qsl-jit --all-features
--test it -- handle records values coverage` (14 passed).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The call arena keeps every handle value until the call returns, so memory grows with executed operations, not with live values, and no limit bounds it | qsl-jit/src/kernel.rs:43, qsl-jit/src/kernel.rs:98, qsl-jit/src/prepared.rs:419 |
| FND-002 | low | `run_value` ends in `_ => return None` over the closed `OpKind`, and `run` lists the same kinds as a run-time `jit-operation-dispatch` fault, so a new kind is a run-time fault instead of a compile error; immediately invoked closures carry the `?` flow | qsl-jit/src/kernel.rs:309-443, qsl-jit/src/kernel.rs:290-303 |
| FND-003 | low | Doc comments still say records are not translated or drawn, while `FORMS` lists `record`, the coverage lists the Record type and the generator draws `GR` records | qsl-jit/src/coverage.rs:33-35, qsl-conformance/src/generator.rs:11-16 |
| FND-004 | low | `div` with no admitted selection is now translated on the handle path and faults at run time, while the same case for `rem` is listed unsupported at prepare | qsl-jit/src/translate.rs:657 |
| FND-005 | low | `records.rs` lists `getx` twice in `FUNCTIONS`, so one function runs twice and the array length overstates the set | qsl-jit/tests/it/records.rs:33,44 |

## Verdict

No soundness defect in the handle path. Handles are indices into a
`Vec<Value>` owned by one call's `EntryFrame`; entries are never removed or
mutated, so a handle cannot dangle or alias within a call, and a fresh
`State` per call means no handle survives into another call. Each use checks
the index against the arena and the value kind, and a mismatch is a typed
fault, not a panic. Generated code holds only indices, never pointers, so
arena reallocation is safe. No new `unsafe` or `extern "C"` code exists
outside `exec`; `EntryFrame` offsets still come from `offset_of!`, so adding
`state` keeps the FR-568 check meaningful. Stops discard losses as the
interpreter does. Decimal losses are recorded at the operation's location
through the same `evaluate_decimal` call. ConvertDecimal from a Rational
divides numerator by denominator exactly as the interpreter does. The
Literal, Field, Stage, Box and Unbox steps charge nothing, matching the
interpreter. The evaluator's Record, Tuple and Field arms delegate to
`composite` with the same fault identifiers and the same order of steps.
The rewritten `invariants.rs` assertion still pins the count, zip and
missing-value proof, now in `composite.rs` through `include_str!`, and still
checks that the evaluator pops exactly `present_slots(slots)` values. Staging
is LIFO and consumed by the construction that follows, including nested
constructions and calls inside operands. FND-001 is the only finding above
low.

FND-001 detail: the interpreter drops each intermediate value after use. The
JIT pushes every Literal, arithmetic result, Field read, Box and construction
result into `State::arena` and frees nothing until `call_metered` returns.
Example: a function returning a 16-scalar Text literal, called inside a body
that the work-unit limit lets run 10^7 times, keeps 10^7 Text copies alive in
the JIT and about one in the interpreter. Literal, Stage, Field and Box charge
nothing, so the meter does not bound this growth either. Fix options: bound
the arena with a resource limit that stops with a typed outcome, or reclaim
entries (for example, truncate to a watermark when a callee returns its
result).

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, afc38c37f. Each call to a callee that allocates takes a `Mark` before its arguments, and a `Reclaim` after `leave_on_failure` truncates the arena back to that mark. A result at or above the mark (a callee value or an argument passed through) moves down to the mark. A result below the mark is the caller's own and stays. Nothing the caller still uses lies between the mark and the call, because only the call's own arguments are emitted there. `staged` holds values, not handles, so it is untouched. Nested marks follow stack order. A stop skips the Reclaim and drops the whole state. `settle` faults with `jit-arena-bound` above `ops.len() + arguments.len()`. Without loops or recursion, each op adds at most one live handle per activation, so a legal program cannot reach the bound. Verified by reading the code, `cargo test -p qsl-jit --all-features` (all green, including the 300-unit parity run) and clippy with `-D warnings`. |
| FND-002 | fixed | Round 1, afc38c37f. `kernel::run` is now one exhaustive match over `OpKind` with no wildcard, no `run_value` and no immediately invoked closures. |
| FND-003 | fixed | Round 1, afc38c37f. coverage.rs and generator.rs now name only collection as absent. |
| FND-004 | fixed | Round 1, afc38c37f. `lacks_selection` covers `div` and `rem`, and both the emit path and `division_interval` use it. Unit test added. |
| FND-005 | fixed | Round 1, afc38c37f. `FUNCTIONS: [&str; 12]` lists `getx` once. |
