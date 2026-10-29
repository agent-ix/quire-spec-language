---
id: SR-1787001
title: "code review (with rust-review) of PR #1787 (QSL-548 slice A): FR-146 step simulation records"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1787 git diff origin/terra-qsl-integration...39c8b047f (11 files)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1787001: code review of PR #1787 (QSL-548 slice A)

## Summary

Head 39c8b047f, base terra-qsl-integration. Checked against FR-146 as amended by STD-556. As ruled, the protocol-subject interim is not a finding.

**What holds.**
- S3 writes one `Initial` record and one `Step` record per V2 `steps` entry into `requirements()`. Each is keyed by its O-07 generated occurrence at the refinement node (ordinal 0, or position + 1). An equal record at an occupied key is accepted; any other is `unkeyable`.
- `row_positions` inverts `step_sources`, so `step_row_ref(source)` is the row's position in FR-375's order. A missing or duplicated source is a fault, not a silent 0. That replaces the old caller-supplied sort.
- Records are skipped exactly when `simulation` is `None`: a protocol subject, a history field, a hidden field, an `any` row, or an ambiguous context.
- A `Step` record's type roots are the receiver, result, parameters and `_`-argument domains, keyed under the concrete operation's anchor. Its populations are the mapping's reaches, the context population and the operation's contract-clause populations. An authored bound adds no domain, through the shared `record()`.
- An ambiguous receiver refuses `ambiguous_declaration`/`ambiguous-name` at its row, naming the populations ascending. An ambiguous mapped type with no row refuses at the refinement name.

**FND-001 (low).** The code and FR-146 "Claim extent" differ on a `Step` record's contract-clause populations.
- `contract_populations` takes every population of the operation's `Clause` claims: the clause's context population, each population a `reaches` walks, and each FR-391-named population.
- FR-146 says only "each population a `reaches` in the operation's contract clauses ... walks".
- The code's reading is the safe one: the obligation reads the clause bodies, so a named population is ranged over. Either the spec text should say "each population the operation's contract clauses range over (FR-104, FR-391)", or the code should be narrowed. I recommend the spec edit.

**FND-002 (low, spec question).** An `Initial` record ranges only over the populations covering the mapped concrete types, as FR-146 states. But the `Initial` obligation evaluates the state mapping. A `reaches` in it walks a population, which every `Step` record includes (`roots.mapping`) and `Initial` does not.
- If that population covers no mapped type, the `Initial` extent omits a domain its obligation ranges over.
- This needs confirming with the FR-146 owner. Most likely `Initial` should take `roots.mapping` too.

**Rust.**
- `object_node_of` factors out the lookup `population_domain_key` used.
- `RowRoots.context: Result<Option<PopulationDomain>, Vec<DeclarationKey>>` carries an error inside a value. It is consumed once, in `simulation_roots`, so it is acceptable.
- `fault()` reports `UnkeyableRequirements` for every structural mismatch. It is internal-only, so that is fine.

**Gates** at 39c8b047f (`--all-features`, debug info off, QSPEC_DIR = qspec-63ff21f1):

| Suite | Result |
| --- | --- |
| qsl-semantics it `refinement` | 93 passed |
| qsl-semantics lib `refinement step_simulation` | 41 passed |
| qsl-replay `refinement_history` | 28 passed |
| qsl-analyze `refinement_request` | 4 passed |
| clippy (3 packages) and fmt | exit 0 |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A `Step` record takes every population of the operation's contract clauses (context, reaches, FR-391-named); FR-146 names only reaches-walked ones. The code's reading is the safe one, so the spec text should follow | FR-146 Claim extent |
| FND-002 | low | An `Initial` record omits the populations a `reaches` in the state mapping walks, which its obligation evaluates; spec question for FR-146 | FR-146 Claim extent, ADR-020 RM-2 |

## Verdict

Mergeable. Both findings are low and spec-facing.
