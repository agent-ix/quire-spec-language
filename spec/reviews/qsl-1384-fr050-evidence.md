---
id: SR-1957031
title: "Evidence review — FR-050 checked successors"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; FR-050-AC-1..8, TC-150, checked adapter and producer/reader/writer tests at PR #1973 head 95ec6998923cbb58fb831a1422f99505390ad6d9; disposition round 4 reviewed at b365e72bcc6de760b36b40bcdabc35f1699c11ec"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-150
    type: reviews
---

## Summary

Test is the declared and appropriate verification method for the checked adapter and strict artifact path. TC-150 provides strong evidence for parameter mismatch refusals, pre-position authentication, progress watermark retention, strict cross-version reads, and writer handoff. Three evidence slices remain incomplete.

## Verdict

**FAIL** — producer-limit coverage, exact-locus negative vectors, and authenticated activation-progress evidence are incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-5's mapped limit/retry test measures only `read_v2_bytes`/`read_v2`; it does not run `native::admit_v2` at exact and one-short producer bounds. | FR-050-AC-5, tests/it/compiled_protocol_v2.rs:1971-2040 |
| FND-002 | medium | The exact checked-to-admitted source-locus contract is negatively tested only by changing source authority; source identity, declaration name and byte-span substitutions are not independently asserted. | FR-050-AC-6, tests/it/compiled_protocol_v2.rs:826-856 |
| FND-003 | medium | The TC-150 checked progress control uses `on origin` with no captures, so it cannot detect that the `/2` progress path drops the retained-instance map. | tests/it/compiled_protocol_v2.rs:656-671, src/temporal.rs:208-223 |

## Coverage

- Manually checked all FR-050-AC-1..8 trace tags and TC-150 procedure steps against tests/it/compiled_protocol_v2.rs and tests/it/handoff_writer.rs.
- `quoin advise --json` (Quoin 0.29.1) produced no stdout/stderr for 30 seconds and was interrupted; authored verification methods remain `Test`, and the method/evidence match was manually checked.
- Focused strict reader/producer refusal tests and writer tests passed; the missing slices are test coverage gaps, not missing methods.

## Dispositions

Round 4, reviewed at head b365e72bcc6de760b36b40bcdabc35f1699c11ec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Producer limits now run for every exposed non-zero dimension and compare retry usage and bytes. after_excerpt: `assert_eq!(producer_retry.bytes(), producer_admitted.bytes()); assert_eq!(native::admit_v2(proofs, selected, &tables.producer, Limits::default()).usage(), producer_usage, "fresh producer retries start with reproducible usage");` |
| FND-002 | fixed | All four source-locus mismatches require typed refusal before position work and no authenticated result. after_excerpt: `for (axis, package) in [     ("source authority", &wrong_authority),     ("source identity", &wrong_identity),     ("declaration name", &wrong_name),     ("declaration byte span", &wrong_span),` |
| FND-003 | fixed | The authenticated progress test now verifies one-time captures across replay/restoration and typed eviction incompleteness. after_excerpt: `assert_eq!(restored_assessment.capture_evaluations, 1); assert_eq!(restored_assessment.captures[0].value, "7");` |

## Final disposition

**PASS — clean at `b365e72bcc6de760b36b40bcdabc35f1699c11ec`.** Round 4 verified all findings in the pass-3 record as fixed and found no new findings. The original `## Verdict` and `## Findings` sections above preserve the pass-3 results; the round-4 outcomes are recorded in `## Dispositions`.
