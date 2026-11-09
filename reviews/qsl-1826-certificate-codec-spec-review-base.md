---
id: SR-20261109172912000031
title: "spec-review/base \u2014 QSL-1826 certificate codec"
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md;
  FR-201/FR-282 closed unbounded profile
review_set: subset
---

## Summary

Base review of the FR-255 engine-budget table delta: three certificate byte/working-storage/rank mappings and their existing owning requirements. No new specification defect found in this narrow delta.

Frozen head b8e90511601149793afae8ddcfb626f3f6a056b3, base 91c6158a171f335ba51dbc129f936e5b7a787096. Ticket: QSL-1826. Review scope is untimed, unbounded acyclic exact eventual reachability only; bounded monitor implementation, broader certificates, malformed replay-wire forms and refutation/witness work remain open.

## Verdict

**PASS for the edited table only.** The additive document fields and limit names map to existing FR-201/FR-282 behavior, distinct byte/storage defaults are finite 1073741824 bytes, and rank uses the existing Count maximum 9007199254740991 with the representation-limit qualification stated in its row. Existing requirement identity/relationships and AC numbering are unchanged. No authored requirement statement, new relationship edge, domain object or executable language was introduced. No new hash/pin/tracking obligation occurs in the delta. Existing broader requirements and inherited test references are context, not newly fulfilled scope. Code defects are recorded only by code-review.

## Assurance Context

No active AssuranceProfile applies: repository search found mentions in historical review artifacts, not a current profile declaration. No active exception was used. No enforced review workflow was required. Full computed matrix and aggregate architecture/string-edge/deny gates were not run under the explicit scoped-review brief; no full assurance completion is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined units (clean units included):

```yaml
- id: FR-255
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: examined
  excerpt: '| `certificate_check.max_certificate_bytes` | `max_certificate_bytes`
    | certificate checkers | `CertificateCheckLimits` max_certificate_bytes; EN-5
    `ExactProbLimits.document.bytes` | 1073741824 bytes | FR-245, FR-201 |'
- id: FR-255
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: examined
  excerpt: '| `certificate_check.max_working_bytes` | `max_working_bytes` | probability
    document writer/reader | `ExactProbLimits.document.storage_bytes`; source checker
    document storage ceiling | 1073741824 working bytes | FR-201, FR-282 |'
- id: FR-255
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: examined
  excerpt: '| `certificate_check.max_rank` | `max_rank` | exact reachability document
    writer | closed-profile Count representability ceiling; a stop requires a different
    supported representation, not a larger Count | 9007199254740991 rank units | FR-201
    |'
```

## Method Selection

Ran base and integrity for the FR-255 table delta. EARS was not applicable because no requirement statement changed; dependency because no relationship edge changed; object/scope-boundary because no domain object or ownership boundary changed. Additional failure-domain, evidence, risk-complexity and criterion-strength lenses were not needed for the edited table. The implementation's source/resource failure scenarios were checked in the Rust lane. The full computed matrix and aggregate gates were excluded by the scoped brief; this is not full repository assurance.
