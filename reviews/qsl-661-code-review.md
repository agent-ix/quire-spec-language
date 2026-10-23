---
id: SR-12356
title: "code review and Rust review of QSL-661 encode_smt_query (QSL PR #1287)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@39e739bf3a4b9cff0ef63ae985d7c09ea933ddff; qsl-eval/src/smt/, qsl-eval/src/simulation/model/symbolic.rs, qsl-replay/src/model_check/smt_query.rs, qsl-eval/tests/it/smt_query.rs, qsl-analyze/tests/it/smt_query.rs"
review_set: subset
---
# SR-12356: code review and Rust review of QSL-661 encode_smt_query (QSL PR #1287)

## Summary

Ticket: QSL-661 (slice E-SMTe). This review covers FR-315 `encode_smt_query` against
FR-315-AC-1 to AC-5 (TC-900). It covers the canonical printer, the state-copy layout, the
expression reader, `StepRelation`, the monitor table, the symbolic subject view and the
`qsl-replay` facade. The rust-review lane is folded into this file.

The split is at a clean boundary. AC-6 is the "Refinement queries" section and AC-7 to AC-11 are
the "Hyper self-composition queries" section. Neither is touched here.

Probes were run on this head and are not committed. A scratch test wrote the scripts out, and z3
5.3.1 (pip `z3-solver`) checked them:

- P1 is `Counter` `Int[0,3]` where `inc` has no `pre` and has post `value - pre(value) = 1`,
  under infinite-trace, with `always holds(c.value <= 2)`. `Unrolling{3}` is sat, because value 3
  is reachable at step 3. `Unrolling{4}`, `InductionBase{4}` and `InductionStep{4}` are all unsat.
- P2 is the AC-1 `Counter` fixture under the bounded false-extension profile.
  `always[0,5] holds(c.value <= 2)` gives `Unrolling{3}` sat but `Unrolling{5}` unsat.
  `always[0,4] holds(c.value <= 3)` gives `Unrolling{4}` unsat.

`cargo clippy -p qsl-eval -p qsl-replay --all-targets -- -D warnings` is clean, and the smt_query
tests pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Unsound `proved`. The terminal stutter holds only where every operation guard is false (`enabled` is the guard conjunction). FR-120 calls a state terminal when it has no successor (`terminal_class(state, has_successor)`). A state where some guard holds but no post-state satisfies the post and the frame therefore has no SMT successor at all. Every path through it is cut, so a violation at or after it never reaches `depth`. Probe P1 shows this: `InductionBase{4}` and `InductionStep{4}` are both unsat for a claim refuted at step 3, so k-induction certifies a false claim. `Unrolling{4}` is also unsat. A sound fix is to let the stutter disjunct hold unconditionally under infinite-trace. That over-approximation adds no new reachable state, and pairwise distinctness already removes it from the step case. | qsl-eval/src/smt/relation.rs:99-112 |
| FND-002 | high | Unsound `proved` for the finite-trace (bounded, false-extension) profile. Every step `0..depth` must take an operation disjunct, so a trace that ends before `depth` drops out of the query. The violations on that trace are lost. So is the false-extension rejection of `always[0,u]` past a terminal state (`BoundedMonitor::terminal`: an `Open` state with remaining > 0 rejects). Probe P2 shows this: `always[0,5] holds(c.value <= 2)` is unsat at depth 5 although it is violated at position 3. | qsl-eval/src/smt/query.rs:340-351; qsl-eval/src/smt/query.rs:275 |
| FND-003 | medium | `InductionStep` never reads the guard definedness at `depth`. `negated` skips `guard_failure` when `last`, and `InductionBase{k}` reads guards only at `0..k-1`. A reachable state at distance `>= k` whose guard is undefined is therefore covered by neither query, and the base/step pair can prove a claim that UE-1 refutes. The defect is latent today, because S3 rejects unproved `value` reads (see SR-12357). It becomes live as soon as any partial guard construct type-checks. | qsl-eval/src/smt/query.rs:494-527 |
| FND-004 | medium | `post_failure` (enabled, frame, and an undefined post) is read on the `(s_i, s_{i+1})` pair that the transition assertion also constrains. An undefined post makes its own disjunct false. The failure is therefore found only when some other disjunct yields the very same successor, and an undefined post on an FR-120 candidate (`ContractUndetermined`) is missed. Let the failure disjuncts satisfy the transition, or read them over free next-state constants. This is latent under S3, like FND-003. | qsl-eval/src/smt/relation.rs:219-223; qsl-eval/src/smt/query.rs:516-521 |
| FND-005 | medium | No resource accounting. `MAX_CONSTANTS` bounds declarations only, and none of the following is charged against the subject's meter or `ExpansionLimits`. `argument_vectors` materialises the full product of the parameter domains, and `Domain` values are collected in full (an `Int[0, 2^40]` parameter is enough). `assert_distinct` is O(depth^2) terms over every field, monitor moves are up to 2^8 x 64 per position, and `distinct()` is O(n^2) over the negated disjuncts. Adversarial input can exhaust memory or hang, where the explicit-state walk charges a budget per vector. | qsl-eval/src/smt/relation.rs:147-170; qsl-eval/src/smt/query.rs:184-201,408-424,141-149 |
| FND-006 | low | For a bounded `eventually[0,u]` with `depth < u`, the violation is the atom being false at every position `0..=depth`, which is not a violation while positions `depth+1..u` remain. That gives a spurious sat, in the refutation direction. A definedness failure is also read at every position after the atom already held, and the bounded monitor does not read those positions. | qsl-eval/src/smt/query.rs:308-314,498-525 |
| FND-007 | low | `symbolic_operations` drops a `pre` or `post` clause whose name has no `state_clause` (`filter_map`), when it should report `Fault`. The `ObjectLifecycle` refusal for a mismatched initial state names `rows.first()`, not the offending object. | qsl-eval/src/simulation/model/symbolic.rs:144-147,263-273 |
| FND-008 | low | `StepRelation`, `StepDisjunct` and `StepLabel` are re-exported `pub`, and the PR changelog lists them, but `StepRelation::build` is `pub(crate)`, so no caller outside `qsl-eval` can obtain one. `StateCopy` has only `at(position)` (`s<i>`), and QSL-1010's `s<i>.<t>.<object>.<field>` copies need a trace-variable prefix. Either export the types `pub(crate)` or widen `build` and `StateCopy` together when QSL-1010 lands. | qsl-eval/src/smt/mod.rs:30; qsl-eval/src/smt/layout.rs:15-23 |
| FND-009 | low | The module doc of `query.rs` says each transition assertion is "followed by its monitor steps". The code asserts every transition and then every monitor step, including the `m0` initial step. The doc and the printed order disagree, and FR-315 "in the order above" leaves the order unpinned. | qsl-eval/src/smt/query.rs:5-12,340-354 |
| FND-010 | low | Constant names are joined with `.` unescaped. Object `a.b` with a field `present`, and object `a` with an optional field `b`, both print `s<i>.a.b.present`. The result is a duplicate `declare-const`, which the solver rejects, when the encoding should refuse `ObjectNameClash`. | qsl-eval/src/smt/layout.rs:175-185 |

## Verdict

Not mergeable. FND-001 and FND-002 are reproduced soundness defects: the encoder produces
unsatisfiable proof queries for claims that the explicit-state checker refutes. Because FR-314
certifies a refutation of exactly these queries, a `certified proved` would follow. The PR body
lists both as "known gaps", but a known unsound `proved` is still unsound. Each must be fixed or
refused in this PR. A refusal would cover a subject with an operation whose post can be
unsatisfiable while its guard holds, and the finite-trace profile, until FR-315 defines the
reading.

What is right:

- Canonical printing (decimal literals, `(- n)`, single spaces, one command per line).
- Deterministic ordering, with no hash maps.
- Linear-only terms (`times` refuses non-literal products).
- The definedness of `and`, `or`, `implies` and `if`.
- Coerce range definedness.
- The k-induction simple-path constraint.
- The monitor tabulation, which has one rejecting state, `m<i>` is the state after letter `i`,
  and `m0` is unconstrained in the step case, an over-approximation that keeps the step case
  sound.
- Refusals for every out-of-fragment form.
- A thin facade.

No panics were found on the reachable paths (indexing is bounded, and shifts are below 2^8).

## New findings (disposition pass 1)

Disposition pass 1 reviewed head `784202027`, which includes fix commits `4e7651d6e` and
`4225fca76`. It was judged against FR-315 as STD-338 (#1289, merged as `9fbb0c208`) states it.
The z3 probes were re-run on that head. Probe P5 is a `Counter` that gains `id: Integer`, which
`inc` writes, with `post Id { self.id - pre(self.id) = 1 }`. Under infinite-trace,
`always holds(c.id <= 0)` is violated after one `inc`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | high | Unsound `proved`. `Application::writes` counts a field as written only when it has a finite domain, so a frame-granted `Integer` field is held equal across every operation disjunct. In probe P5, `id`'s post then contradicts the frame and only the stutter is left: `Unrolling{1}`, `Unrolling{2}`, `InductionBase{1}` and `InductionBase{2}` are all unsat for a claim violated at step 1. FR-315 prints frame equality only for fields the frame does not grant, and refuses `field-domain` only for queries that expand candidates. The step relation must leave a granted field free. | qsl-eval/src/smt/relation.rs:339-345 |
| FND-012 | medium | Refusal row 10 `field-domain` is not implemented. A query that expands candidates (false-extension, deadlock-freedom, a postcondition failure) over an application that writes an infinite-domain field computes terminality over the frozen field instead of refusing. In P5, the deadlock item's `InductionBase{1}` is sat with a spurious deadlock at value 0. | qsl-eval/src/smt/relation.rs:369-423 |
| FND-013 | low | The refusal codes differ from FR-315 Outputs. The single-subject instance check reports `instance-binding` where the spec names `instance-type`. `internal-fault` is a code the spec's "exactly these" list lacks. A finite parameter domain over 2^16 values refuses `parameter-domain`, but by the spec it is the `applications` measure (`too-large`). | qsl-eval/src/smt/refusal.rs:142-157; qsl-eval/src/simulation/model/symbolic.rs:298-300 |
| FND-014 | low | Operations that declare a result are not encoded. No result disjunction exists (FR-315 "Results", AC-24, AC-27), and a clause that reads `result` refuses `out-of-fragment` ("binding with no value"). The refusal is sound but does not conform. | qsl-eval/src/smt/relation.rs:301-335 |
| FND-015 | medium | Canonical printing differs from FR-315 text in ways that change bytes. (a) The false-extension step prints `(and (or (= h<i> h<i+1>) T) (or h<i+1> D))` where the spec has `(and (= h<i+1> (or h<i> T)) (or h<i+1> D))`; the two are semantically equivalent, because a terminal copy has no D-successor. (b) Field equality prints the earlier copy first, where the spec puts the later copy first (stutter, frame, distinctness; AC-15). (c) Optional-field equality is `(and (= V V') (= P P'))` where the spec has `(and (= P P') (or (not P) (= V V')))` (AC-25). This version is stronger: it is sound, but a step-case distinctness can tell apart two equal states. (d) The printer folds `(not (not x))`, `true`/`false` inside `and`/`or`, and comparisons of literals, but the spec says it "simplifies no other term". (e) Distinctness prints `(or (not Q)..)`, not `(not (and Q..))`. FR-314 rebuilds through the same function, so this is not a soundness issue, but the spec pins the bytes. | qsl-eval/src/smt/query.rs:485-495,532-549; qsl-eval/src/smt/layout.rs:320-330; qsl-eval/src/smt/script.rs:124-205 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4e7651d6e. The stutter is unconditional. P1 now gives `Unrolling{4}`, `Unrolling{5}`, `InductionBase{4}` and `InductionBase{5}` sat. `InductionStep{4}` stays unsat by pigeonhole (five distinct copies of a four-valued field), so the coder's note is right. The defect was the unsat base case, not the step case. |
| FND-002 | fixed | 4e7651d6e (the `h<i>` flags), checked with z3. `always[0,5] holds(c.value <= 2)` is sat at depth 5. `always[0,4] holds(c.value <= 3)` is sat at depth 4. `always[0,3] holds(c.value <= 3)` is unsat at depth 3 (AC-18). `eventually[0,3] holds(c.value = 3)` is unsat at depth 3, and `eventually[0,2] holds(c.value = 3)` is sat at depth 2. |
| FND-003 | fixed | 4225fca76. `CopyFacts` is read at every position `0..=depth`, and `InductionStep` includes the precondition and postcondition failures from copy `depth`. This was verified structurally only: S3 refuses the reviewer's partial-guard probe (`Unproved(Range)`), so it cannot be exercised. |
| FND-004 | fixed | 4225fca76. A postcondition failure is read on copy `p` and candidate literals, independent of the transition assertion. |
| FND-005 | still-open | 4e7651d6e added the fixed limits. `distinct()` in `negated` is still O(n^2) over up to about 4 x 2^20 disjuncts, and it runs before the `script-bytes` check, so `Unrolling{2^20-1}` over a one-field subject can hang. Use a set, or skip duplicates that are constants. |
| FND-006 | still-open | FR-315 row 3 requires `query-kind` for `eventually[0,u]` with `Unrolling{depth < u}`, and it is not implemented. `eventually[0,3] holds(c.value = 3)` at `Unrolling{2}` encodes and is sat, which is spurious. |
| FND-007 | fixed | 4e7651d6e. An unresolved clause is now a fault, and `ObjectLifecycle` names the offending object. |
| FND-008 | fixed | 4e7651d6e. `StepRelation` and its types are no longer public. The `StateCopy` trace-variable prefix is QSL-1010's to add. |
| FND-009 | fixed | 4e7651d6e. The module doc now states the printed order. |
| FND-010 | fixed | 4e7651d6e. Dotted keys and fields refuse `constant-name`. |
| FND-011 | still-open | Found in this pass. |
| FND-012 | still-open | Found in this pass. |
| FND-013 | still-open | Found in this pass. |
| FND-014 | still-open | Found in this pass. |
| FND-015 | still-open | Found in this pass. |

### Disposition pass 2

Disposition pass 2 reviewed head `15aefe760`, which carries fix commit `f3668797c`. The z3 probes
were re-run on it:

- P1: `Unrolling{3..5}` and `InductionBase{3..5}` are all sat.
- P2: `always[0,5] holds(c.value <= 2)` at depth 5 is sat, `always[0,4] holds(c.value <= 3)` at
  depth 4 is sat, and `always[0,3] holds(c.value <= 3)` at depth 3 is unsat. `eventually[0,3]`
  at depth 3 is unsat, and `eventually[0,3]` at depth 2 refuses `query-kind`. `eventually[0,2]`
  at depth 2 is sat. `eventually[0,5]` at depths 5 and 7 is unsat. `eventually[0,4]` over a
  counter that stops at 2 is sat at depth 4.
- P5 (`id: Integer`): infinite-trace `Unrolling{1}` and `InductionBase{1}` are sat. The
  false-extension query and the deadlock query both refuse `field-domain` at `c.id`.
- Results (`pick(): Level`, post `self.value = result`): infinite base 1 is sat, step 1 is
  unsat, false-extension `always[0,2] holds(c.value <= 3)` is unsat, and `always[0,2] holds(c.value <= 2)` is sat.
- `InductionStep{2^20-1}` refuses `too-large` `script-bytes` before it builds the pairs.
  `Unrolling{2^20-1}` writes a 257 MB script without hanging.

The printed bytes now follow FR-315: `(= h<i+1> (or h<i> T))`, field equality with the later
copy first, optional equality `(and (= P P') (or (not P) (= V V')))`, distinctness as
`(not (and ..))`, and no folding beyond the condition constants. `cargo clippy -p qsl-eval -p qsl-replay -p qsl-analyze --all-targets -D warnings` is clean, and the smt tests pass.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | f3668797c. A `HashSet` dedupe by printed text, the byte count kept while disjuncts are pushed, and `check_pairs` before the step case builds its pairs. Probed: the 2^20-1 step case refuses at once. |
| FND-006 | fixed | f3668797c. `eventually[0,3]` at `Unrolling{2}` refuses `query-kind`. |
| FND-011 | fixed | f3668797c. `writes` no longer requires a finite domain. P5 is sat at depth 1. |
| FND-012 | fixed | f3668797c. `candidates` refuses `field-domain` for a written field with no finite domain. P5's false-extension and deadlock queries refuse at `c.id`. |
| FND-013 | fixed | f3668797c. The instance check refuses `instance-type`, there is no `internal-fault` code, and an oversize domain counts toward `applications`. |
| FND-014 | fixed | f3668797c. The result disjunction is read in FR-120 result order, and later results fail only where earlier ones are rejected (AC-24, AC-27). The results probe is consistent. |
| FND-015 | fixed | f3668797c. Points (a) to (e) all conform; the AC-15, AC-17, AC-22 and AC-25 byte tests pass. |
