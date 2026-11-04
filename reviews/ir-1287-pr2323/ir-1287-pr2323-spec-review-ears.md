---
id: SR-20261104231206
title: IR-1287 public refinement handoff spec-review-ears
type: SpecReview
analysis: ears-conformance
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

The added normative statements name the replay request or assembly actor, input/condition and concrete response. No changed-statement EARS warning or ambiguous trigger was found. No applicable AssuranceProfile is declared under spec/.

## Verdict

PASS

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

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
