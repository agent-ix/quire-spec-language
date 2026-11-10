---
id: SR-20261110042820830003
title: failure-domain review of long-run source-admission correction
type: SpecReview
analysis: failure-domain
scope: agent-ix/quire-spec-language; QSL-1826 PR2647; spec/functional/FR-201-check-a-probability-certificate.md,
  spec/test-cases/TC-1401-check-unweighted-dtmc-long-run-certificates.md
review_set: subset
---

## Summary

Reviewed correction e8e67c5b0fbb9fdd7ff6e2dfeda045bade6c4543 against dd627eb4ef539a6bee158f2f8ea3302439e98054.

## Verdict

**PASS, specification only** — The corrected zero-population boundary keeps unsupported distinct from vacuous truth. Direct checker binding refuses UnsupportedBinding before considering certificate values; direct EN-5 emits an unsupported disposition with no terminal, certificate or counterexample. Planner behavior is explicitly unclaimed. Threshold source admission and terminal/component values are correctly distinguished.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-201-AC-27
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Both workload bound sides accept at equality; AtLeast 1801/1802 and AtMost
    999/1000 reject ThresholdMissed. These comparison thresholds lie strictly between
    0 and 1 under the existing S3 admission policy. Changing the bias, setting transported
    gain to 0, or replacing long-run evidence by reachability values prevents acceptance.
    No P state absorbs its original transitions.
- id: FR-201-AC-30
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Zero over instances retain UnsupportedBinding at direct claim binding and
    an unsupported disposition at the direct EN-5 item, with no terminal verdict,
    certificate, counterexample or fabricated binding.
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "With zero `over` instances, direct\n  claim binding SHALL retain `UnsupportedBinding`,\
    \ and the direct EN-5\n  item SHALL retain its unsupported disposition without\
    \ a terminal verdict,\n  certificate or counterexample."
- id: TC-1401
  path: spec/test-cases/TC-1401-check-unweighted-dtmc-long-run-certificates.md
  role: examined
  excerpt: "The comparison thresholds\n  stay strictly inside (0,1); no endpoint admission\
    \ change is implied."
```

## Evidence and limits

Inspected actual qsl-semantics/src/check/probabilistic.rs:880–918, qsl-replay/src/probability_certificate.rs:934–957 and qsl-analyze/tests/it/exact_probabilistic_zero_binding.rs at the correction head. Existing differential endpoint fixture cc2293568 and /tmp/qsl1826-long-run-tests3.log demonstrate admission refusal; corrected runtime fixture is not yet qualified. No new builds were run. No active local AssuranceProfile applies and no enforced review acceptance is claimed. Further graph mathematics, dependencies, wire objects and evidence methods were unchanged; their earlier scoped review was not rerun.
