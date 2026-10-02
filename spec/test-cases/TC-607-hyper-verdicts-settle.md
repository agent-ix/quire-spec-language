---
id: TC-607
title: "Hyper and step-relation outcomes settle as terminal records with their causes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: verifies
---
# TC-607: Hyper and step-relation outcomes settle as terminal records with their causes

## Description

Verify the hyper verdict map, the new inconclusive causes, replay-gated refutation, the V-7 budgets, V-8 for HP-4, and the stated reading of a bound.

Scope: FR-182-AC-1 to FR-182-AC-4, FR-182-AC-7, FR-182-AC-8.

## Test Procedure

Fixtures: the outcomes of TC-601, TC-602, TC-604 and TC-598.

1. Map one input of each FR-182 table row.
2. Settle §8.1 leaky and secure, with and without copy-swap; settle §8.2 leaky.
3. Settle the vacuous match, the `max_witness_set` stop, the `max_relation_tuples` stop and the HP-4 clauses.
4. Settle FR-179-AC-3's `max_depth` run.
5. Settle §8.1's secure proof with FR-183-AC-7's accepted certificate and with its member-removed certificate; settle an HP-2 `Inductive{depth: 1}` outcome.
6. Settle the outcomes of FR-177-AC-5, FR-177-AC-6 and FR-178-AC-6.

Tag the tests `#[trace("TC-607", "FR-182-AC-n")]`.

## Expected Results

- Step 1: each row as the table states; `MatchUndetermined` and `VacuousMatch` inconclusive.
- Step 2: `refuted`, `decisive-counterexample` after replay; `proved`, `Exhaustive` and `Reduced` naming `CopySwap`; `refuted`, `decisive-counterexample`.
- Step 3: `inconclusive`, `VacuousMatch`; `failed`, `resource-incomplete` naming each limit and value; `unsupported`, `unsupported-requested-capability`.
- Step 4: `inconclusive`, `BoundReached{depth: 1}`, `completed`, `pending`, with the HP-1 reading.
- Step 5: `proved`, `Certified`; `inconclusive`, `CertificateRejected`; `proved`, `decisive-witness`, `Uncertified`.
- Step 6: `refuted`, `decisive-counterexample`, `UndefinedEvaluation` naming `a` and `b`; `inconclusive`, `MatchUndetermined` twice.
