---
id: SR-20261104231204
title: IR-1287 public refinement handoff gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; IR-1287; PR 2323 retained-request SMT certificate
  adapter
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-492
  type: references
- target: ix://agent-ix/quire-spec-language/FR-314
  type: references
---

## Summary

The six added criteria compute tagged, but two named adverse behaviors lack execution controls: duplicate Base and reservation failure. No applicable AssuranceProfile is declared under spec/.

## Verdict

PASS after disposition round 1. Original findings remain below with their fixed outcomes recorded separately.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The duplicate-record loop always copies original Step 0 (parts[1]), so it exercises duplicate Step records twice and never a duplicate Base. AC-9 explicitly requires duplicate Base and duplicate Step refusals. Add a separate selected-count-preserving duplicate-Base input, such as replacing one Step with the original Base, and assert Shape without removing the existing controls. | qsl-replay/tests/it/refinement_query_facade.rs:362-365 |
| FND-002 | medium | The only AC-11 binder tests byte/record limits, cancellation and independent checker budgets, but never triggers the new fallible-reservation ResourceExhausted path. AC-11 explicitly includes reservation failure. Add a deterministic private real-reservation failure control that exercises the normal assembly result and proves no certificate escapes, without an OOM stress run, fabricated error-only test or public fault API. | qsl-replay/tests/it/refinement_query_facade.rs:433-495 |

## Coverage

Plan completion: not assessed. Examined all changed files and the six added criteria. The computed Quire matrix reports each added criterion tagged. The explicitly requested semantic binding check found the two gap-analysis issues; no repository-wide clean coverage claim is made.

Reviewed supplied focused public/private/compile-fail, Clippy and API-surface receipts; independently ran the computed matrix and scoped specification validation. No Cargo or solver run was performed by the reviewer. The final aggregate gate remains the lead’s responsibility. Genuine CG solver output reaching native Unverifiable for an unsupported proof rule is not adapter verification credit or a new adapter defect; this PR preserves that result and leaves live IR-1120 qualification open.

The parser compares the header count with the retained selected rows before slot allocation, checks framing feasibility, uses fallible reservations, consumes the whole stream and preserves query/proof bytes. A private borrowed Part is a parsing view, not a public mirrored run-record DTO. The shared refinement row join is used by the facade and checker. No new pin, workflow change or internal qsl_eval request export is present.

### Examined units

- FR-492-AC-8 (examined), spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md: Over a replay-root request for the seven-row refinement, the CG-format Base and Step handoff assembles in row order in either input order, with exact query/proof bytes preserved; the zero-row refinement accepts Base only. No caller-provided count defines the selected refinement.
- FR-492-AC-9 (examined), spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md: Missing or duplicate Base or Step, out-of-range Step, a handoff for a different selected row count, unknown tag, truncated header/row/field, impossible length, mismatched count and trailing bytes each refuse with the specified typed shape or malformed result and no partial certificate. The private assembler also refuses absent query or proof fields.
- FR-492-AC-10 (examined), spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md: A present empty proof and arbitrary non-UTF8 bytes survive assembly. The request-owned checker reports the native NotRefutation or Malformed rejection for those proofs and QueryMismatch for a different query; assembly itself claims no proof acceptance. A checked-rule refutation through the facade yields the same native result as FR-314.
- FR-492-AC-11 (examined), spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md: Assembly succeeds at its exact byte/record budgets, stops at one less or zero with the corresponding existing limit and setting, and a cancelled handle stops with its native cause. Limits, cancellation and reservation failure expose no partial certificate. Checking retains its independent FR-314 budget.
- FR-492-AC-12 (examined), spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md: The two new request methods are usable from replay-root types alone; no adapter public signature exposes the internal certificate request, and callers cannot invoke the private with_certificate_request bridge.
- FR-492-AC-13 (examined), spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md: The public refinement_key accessor returns the exact key selected from the retained closure, and step_count returns the authoritative declaration-order row count for seven-row and zero-row selections; an unrelated key faults rather than giving a count. The driver needs no private request type or caller row-count authority.
- FR-314 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: Assembly is not proof verification: empty, malformed and unknown-rule proof bytes must reach the checker unchanged.

### Files examined

- qsl-replay/src/certificate/smt.rs
- qsl-replay/src/lib.rs
- qsl-replay/src/refinement_query.rs
- qsl-replay/src/refinement_query/handoff.rs
- qsl-replay/tests/it/refinement_query_facade.rs
- spec/functional/FR-314-check-an-smt-proof-certificate.md
- spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md
- spec/test-cases/TC-1275-expose-the-refinement-simulation-query-through-the-replay-facade.md

### Method selection

Code review includes the Rust lane. Gap analysis includes the requested intent-to-test check. Base, EARS, integrity and scope-boundary cover the changed normative interface. Object/dependency analyses are not applicable because no new spec object artifact or relationship edge is introduced. No additional reviewer or heavy build was used.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The added selected-count-preserving case replaces Step 3 with the original Base and asserts Shape. Both existing duplicate-Step controls remain unchanged. |
| FND-002 | fixed | A private thread-local one-shot invokes real Vec::try_reserve capacity failure from the normal payload reservation helper. The test exercises failure before copying, after one payload and after the base plus a step query, observes ResourceExhausted with no certificate, consumes the one-shot and verifies the next assembly preserves all bytes. |

### Disposition verification

The native capacity-overflow call cannot request an actual huge allocation. All partially copied payloads remain local Vec owners and drop on the Result error path, so no prefix certificate is published. Production without cfg(test) continues using the same fallible exact reservation; no public fault API, global allocator replacement, cap, proof rewriting or checker change was introduced. No new findings. Reviewer ran no Cargo; the final aggregate gate remains with the lead.
