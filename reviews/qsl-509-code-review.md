---
id: SR-12125
title: "Code review of quire-spec-language PR #954: TemporalTrace evaluator over a closed trace, a finite prefix and a lasso"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a193cde9cde16886dd343352eafe7b3f38d93336; PR #954 (git diff origin/main...HEAD, 17 files): qsl-eval/src/temporal/{mod,engine,input,letters,outcome,plan,position}.rs, qsl-eval/src/lib.rs, qsl-eval/tests/it/temporal_trace.rs, qsl-eval/tests/it/main.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/temporal_clause.rs, spec/decisions/ADR-018, spec/functional/FR-125, FR-126, FR-329, FR-338"
review_set: subset
---
# Code review of quire-spec-language PR #954

## Summary

Ticket: QSL-509 (T3a). Code review with the rust-review lane folded in.
Read-only review: no build and no tests were run. The author's full-ci
chain on this head is taken as reported.

The evaluator reads every trace as one infinite word. A lasso is its own
word. A finite trace is followed by a one-position sentinel loop: false
under false-extension and Unknown for an infinite-trace prefix. Truth is
three-valued (Kleene). The SM-8 unrolling uses `m = ceil(R/L) + 1` copies,
and the past reach follows SM-8 term by term: future operators add 0, a
closed past interval adds `b`, and an open past interval adds `a + L`.
Positions past `N` read from the last copy by congruence. Positions before
0 read atoms false, through a per-node left floor. Evaluation runs on an
explicit frame stack with a (node, canonical position) memo, so it has no
recursion and an unbounded window cannot loop.

What was traced and found correct:

- Lasso evaluation, including past conditions across the loop edge.
  `once[1,1]` at position 3 reads position 2. An unbounded `once` reaches
  back into the prefix (FR-329-AC-1, AC-2).
- Window truncation at `max(start, N) + L - 1` and the wrapped `[a,*]`
  window. Both are sound because every subformula is periodic from
  `n + R(psi) <= N - L`.
- The open until/since windows start at `i+a` (and `i-a`) with the guard
  over `[i+a, d)` (and `(j, i-a]`). This matches the amended ADR-018 IV-2,
  FR-125, FR-329 and FR-338, and QSpec FR-367 on quire-specification
  origin/main. The `UnsettledOpenInterval` refusal from 7c55318b3 is gone
  entirely, and nothing in code still implements the old prefix-debt
  reading.
- The bounded profile refusals. A lasso under a bounded profile gets
  `LassoUnderBoundedProfile`. The timestamped profile gets
  `ProfileNotEvaluated`.
- `MissingFairnessPremise`. It comes before any read or charge for both a
  prefix and an observed lasso.
- Undefined letters. Activations are cut at the first undefined position,
  and an earlier false activation wins.
- The safety-fragment test, done in NNF polarity.
- u64 and i128 arithmetic. Prefix plus loop overflow gives
  `TraceTooLong`, unrolled positions are converted with a checked
  `try_from`, and spans are checked.
- Empty loop and empty finite trace each refuse before any read.

Production code has no `unwrap`, panic or raw indexing.

Tests assert typed outcomes and typed `Incomplete` fields. The
`#[trace(...)]` tags sit only on ACs that the tagged test checks.
FR-329-AC-3 (`Lasso::Model` half) and AC-4 are correctly left untagged.
The PR adds no compat layer.

## Verdict

Approve, no blocking (high) findings. Three mediums, recorded. Work
accounting differs from TR-5's "(temporal node, position) visit" count.
Scans over represented positions are unmetered. FR-328's completeness SHALL
is stronger than the Kleene reading the code implements and ADR-014 A-4
allows. Three lows, recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Work accounting is not TR-5's count. FR-327 "Work" and ADR-014 TR-5 charge once per visit of a *temporal node* (a temporal operator or a `holds` atom) at a position. `frame()` charges every Boolean connective too (`not`/`and`/`or`/`implies`). `advance_scan` also charges one unit per *scan step* over an unrepresented position, keyed on the frame and not on (node, position), so two frames that scan the same unrepresented position pay twice. Example: `always[0,3] not holds(c.level > 2)` `on origin` over the 3-position closed trace is 4 visits by FR-327 (the `always` at 0 and the atom at 0, 1 and 2). The engine charges 9 (3 atom reads, 1 `always`, 4 `not` frames, 1 unrepresented step at position 3). The Incomplete boundary therefore differs from the spec. Either state the extra charges in FR-327/TR-5 as target design, or charge only temporal-node visits. | qsl-eval/src/temporal/engine.rs:648-669; qsl-eval/src/temporal/engine.rs:779-787; qsl-eval/src/temporal/engine.rs:532-534 |
| FND-002 | medium | The meter does not bound evaluation time. A scan step over a *represented* position is free, so `always always holds(p)` (or any nested unbounded/long-window scan) over N positions does about N^2 steps for about N charges. With an atom-free formula, `read_letters` loops `0..represented` with no charge at all, and so does the `on each` activation loop when the root is a constant. A caller-supplied `TemporalTrace::Finite{len}` sets that length. Charge every scan step, or bound the steps by charged units. | qsl-eval/src/temporal/engine.rs:443-477; qsl-eval/src/temporal/engine.rs:531-562; qsl-eval/src/temporal/engine.rs:774-806 |
| FND-003 | medium | FR-328 says: "When the clause is in the safety fragment and every infinite extension of the prefix makes it false ... SHALL return Completed(false)". The engine's Kleene evaluation is sound but incomplete. `always[1,1] holds(c.level = 0) and always[1,1] not holds(c.level = 0)` over a one-position prefix is false in every extension, yet each conjunct is Unknown at position 1, so the result is `Pending`. Separately, `on each` activations past the prefix are never evaluated, even when a later activation is false in every extension. ADR-014 A-4 says "sound and incomplete", so the defect is FR-328's statement. It should state the three-valued (informative-prefix) rule the code implements, or the code needs an exact bad-prefix decision. | qsl-eval/src/temporal/engine.rs:501-527; spec/functional/FR-328-evaluate-an-infinite-trace-clause-over-a-finite-prefix.md |
| FND-004 | low | `TemporalLetters` docs say "the evaluator charges one work unit for the read itself". `read_letters` charges each atom read but not the `when` activation read, so a guarded `on each` clause's condition reads are uncharged by the evaluator. | qsl-eval/src/temporal/engine.rs:445-457; qsl-eval/src/temporal/letters.rs:156-160 |
| FND-005 | low | FR-327/FR-328/FR-125/FR-329 AC texts name `c.value`, which S1 rejects as a keyword. The tests use `c.level` (commit 42cfe0d70). The spec should state the runnable form. FR-329-AC-7's "the same clause" is ambiguous: AC-6's clause would be `Undefined`, not `Completed(false)`. The test reads it as AC-4's `always eventually holds(b.versionNumber = 2)`. | spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md; spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md; qsl-eval/tests/it/temporal_trace.rs:629-662 |
| FND-006 | low | `once[a,*]` before the origin. QSpec FR-367 (origin/main) says `once[a,*] p` "holds when p holds at some j with 0 <= j <= i - a, and is false when i < a". The engine follows FR-092's complete-history rule: atoms are false before 0, but a negated operand is true there. So `once[1,*] not holds(p)` at position 0 is true here and false by FR-367's sentence. QSL FR-125/FR-328 choose the atoms-false reading. The QSpec sentence needs aligning upstream. | qsl-eval/src/temporal/engine.rs:721-727 |
