---
id: SR-15430
title: "Integrity review of STD-239 PSC-7 (c) eco assumption (QSL PR #1199, QSpec PR #352)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@f6fab4b8a8db23c1de3a8db5aff116fe41ec168b; spec/decisions/ADR-025-weak-memory-models-for-parallel.md; agent-ix/quire-specification@cf75bb2c411e08187e3c67bc565c7b2d7034beaf; spec/functional/memory/FR-437-explore-release-acquire-memory.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
---
# SR-15430: Integrity review of STD-239 PSC-7 (c) (QSL PR #1199, QSpec PR #352)

## Summary

Ticket: STD-239. This review covers `git diff origin/main...HEAD` of both PRs,
each a single commit. QSL #1199 at `f6fab4b8a8db23c1de3a8db5aff116fe41ec168b`
changes one ADR-025 PSC-7 row (line 602). QSpec #352 at
`cf75bb2c411e08187e3c67bc565c7b2d7034beaf` changes FR-437's omitted-clause
bullet (lines 169-177). Both add "`P` builds no edge for the `eco` part" and
say the (c) assumption is "checked by script over about 460,000 random litmus
programs of up to 8 events, none with an RMW or a non-atomic access".

Units examined: ADR-025 PSC-7 (c), and FR-437 Behavior "Omitted clauses" (c).

Evidence checked: the scripts in the session scratchpad
(rv-std239-d1/litmus/). fuzz2.py over seeds 11-14 in both modes gives
n = 460,154 programs of at most 8 events, which is the figure the PRs cite.
It checks two things: `rc11(p) <= run_ra(p)`, and
`run_ra(p) <= rc11(p, variant='P7')`. lit.py's `P7` variant builds
`pf = hb` between fences only (lit.py:218), so it drops the `eco` part (c)
along with (a) and (b). If `ra` admitted an outcome that only the `eco` part
forbids, fuzz2 would count that program as `ra-only` and report nothing. The
check that targets (c) is econ.py/eco3n.py (`rc11(p, noeco=True) - rc11(p)`
intersected with `ra`). Over 196,961 programs it found 2 programs where the
`eco` part changes an RC11 outcome, and `ra` excluded both.

Methods: spec-review/spec-integrity-analysis ran. code-review, rust-review
and gap-analysis were skipped because the diffs change no code. spec-ears-analysis
was skipped because neither diff adds or changes a SHALL statement.

Gates on the PR heads, in temporary worktrees: QSL `make
check-spec-validation` exits 0. QSpec `quire validate --scope .
'spec/**/*.md'` gives 86 failure lines (the expected baseline).
`cargo test --test checked_package_v2 --test backend_provider_v1` passes
(14 + 117 tests). QSpec #352 changes only spec prose, and no QSL test parses
it, so it needs no QSL conformance run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | PSC-7 (c) says the `eco` assumption "is checked by script over about 460,000 random litmus programs". The 460,000-program run (fuzz2.py, n = 460,154) compares `ra` with an RC11 variant that also drops the `eco` part (lit.py `variant='P7'`: `pf = hb` only). So that run cannot fail when `ra` admits an outcome the `eco` part forbids. It is evidence for the next sentence's two claims, not for (c). The check that targets (c) (econ.py/eco3n.py) covers 196,961 programs, and in only 2 of them does the `eco` part change an RC11 outcome. Attribute the 460,000 figure to the soundness and only-omitted-edges claims. For (c), cite the targeted check with its real size and its 2 relevant programs, plus the two exhaustive families. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:602 |
| FND-002 | high | FR-437's omitted-clause (c) has the same misattribution as FND-001: "That the join of `S` carries it is not proved; it is checked by script over about 460,000 random litmus programs ...". The run behind that number cannot detect an `ra` outcome that only the `eco` part forbids. Fix it the same way as ADR-025 so the two documents stay in step. | spec/functional/memory/FR-437-explore-release-acquire-memory.md:172-175 |
| FND-003 | low | The added sentence "`P` builds no edge for the `eco` part." repeats PSC-3 (line 598, "builds ... no edge for its `eco` part") and PSC-7's lead sentence, which already says that `P` does not build these edges. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:602 |
| FND-004 | low | The added sentence "`P` builds no edge for the `eco` part." repeats the Behavior bullet 15 lines earlier ("builds its `hb` part as the fence-to-fence edge above and no edge for its `eco` part", line 156-157). | spec/functional/memory/FR-437-explore-release-acquire-memory.md:172 |

## Verdict

Not mergeable as written. Two high findings, FND-001 and FND-002, one per PR,
and both are majors. The new text attributes the 460,000-program script to the
`eco` assumption, but that script cannot fail on it, so the stated evidence
for (c) is overstated. The rest of each edit is accurate: "not proved", the
absence of RMW and non-atomic accesses, and the 8-event bound all match the
scripts. The two documents agree. FND-003 and FND-004 are minor.

## Dispositions

Disposition pass 1 covers QSL PR #1199 at
`2c781b6a7050027c937936a51e6ea7bc5a5b78fc` and QSpec PR #352 at
`8383e03fac644d778244ea098820e90d4e27e716`. Each fix is commit `0ca3a47b1`
(QSL) or `baa0283` (QSpec), followed by a merge of `origin/main`. The figures
were checked against the script outputs in rv-std239-d1/litmus/:

- econ.py and eco3n.py tried 196,961 programs in total. Their filter keeps
  programs of at most 8 events with at least two `seq_cst` fences, and no RMW
  or non-atomic access. In 2 programs the `eco` part changes the RC11
  outcomes, and `ra` admits none of those outcomes.
- fuzz2.py ran 460,154 programs of at most 8 events.

ADR-025 PSC-7 (c) and FR-437 (c) now say the same thing.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | PSC-7 (c) now cites the targeted run with its real size and its 2 relevant programs, plus the two exhaustive families. The 460,000 figure now goes to the two claims it supports, and the text still says "not proved". After: That `S` carries it is not proved. It is checked by script over 196,961 random litmus programs of up to 8 events with at least two `seq_cst` fences and no RMW or non-atomic access, in which the `eco` part changes the RC11 outcomes of 2 programs and `ra` excluded the outcomes in both, and by exhaustive enumeration of two bounded families of programs with fences ... Both claims were checked over about 460,000 random litmus programs of up to 8 events, without RMW or non-atomic accesses, and are not proved. |
| FND-002 | fixed | FR-437 (c) has the same fix, in step with ADR-025. After: That the join of `S` carries it is not proved. It is checked by script over 196,961 random litmus programs of up to 8 events with at least two `seq_cst` fences and no RMW or non-atomic access, in which the `eco` part changes the RC11 outcomes of 2 programs and `ra` excluded the outcomes in both, and by exhaustive enumeration of two bounded families ... both claims were checked over about 460,000 random litmus programs of up to 8 events without RMW or non-atomic accesses and are not proved. |
| FND-003 | fixed | The repeated sentence is removed from ADR-025 PSC-7. After: ... so that a message choice completing the `eco` path is not offered. That `S` carries it is not proved. |
| FND-004 | fixed | The repeated sentence is removed from FR-437 (c). After: ... by not offering the message choices that would complete it. That the join of `S` carries it is not proved. |
