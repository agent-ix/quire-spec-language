---
id: SR-20261109172912000032
title: "spec-review/integrity \u2014 QSL-1826 certificate codec"
type: SpecReview
analysis: integrity
scope: agent-ix/quire-spec-language; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md;
  FR-201/FR-282 closed unbounded profile
review_set: subset
---

## Summary

Integrity review of FR-255's added/expanded certificate budget table rows against the closed document contract and existing limit vocabulary. No new table-level contradiction or ambiguous unit found.

Frozen head b8e90511601149793afae8ddcfb626f3f6a056b3, base 91c6158a171f335ba51dbc129f936e5b7a787096. Ticket: QSL-1826. Review scope is untimed, unbounded acyclic exact eventual reachability only; bounded monitor implementation, broader certificates, malformed replay-wire forms and refutation/witness work remain open.

## Verdict

**PASS for the edited table only.** Byte output and canonical-reader working storage use distinct named settings and independent limits, with defaults given in byte units. The fixed Count representability ceiling is explicitly distinguished from a raiseable storage ceiling. Existing FR-201/FR-282 own the behavior and tests; no new US/FR/StR relationship or copied semantic object is required by a table extension. Writer admission implementation does not meet its bound; that concrete source defect is intentionally confined to code-review FND-001. Failure probes covered missing/foreign identity, unsupported shapes and limit refusal without truth; no new spec-level failure-domain concern warrants a separate lens.

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
