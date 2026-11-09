---
id: SR-20261109172912000030
title: "gap-analysis \u2014 QSL-1826 certificate codec"
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md;
  FR-201/FR-282 closed unbounded profile
review_set: subset
---

## Summary

Scoped manual requirement/trace/test/source mapping of the changed exact-document and fresh-process functionality. No separate trace or reverse-ownership defect found; computed matrix evidence is unavailable in this pass.

Frozen head b8e90511601149793afae8ddcfb626f3f6a056b3, base 91c6158a171f335ba51dbc129f936e5b7a787096. Ticket: QSL-1826. Review scope is untimed, unbounded acyclic exact eventual reachability only; bounded monitor implementation, broader certificates, malformed replay-wire forms and refutation/witness work remain open.

## Verdict

**CONDITIONAL — manual scoped mapping only.** Plan completion: not assessed. Full computed matrix was explicitly excluded by the dispatch because of known costly/no-output behavior; no matrix-complete or repository-wide assurance claim is made. Optional semantic-gap fanout was not invoked; ordinary Rust review traced behavior and test oracles.

FR-201-AC-9 is tagged by document roundtrip, malformed rows/rationals/fields/keys, key ordering and structural identity nesting tests. FR-201-AC-10 is tagged by child alterations, objective/identity/binding mutations, changed workload weights and rank rule/locus tests. FR-201-AC-11 is tagged by min/max/workload thresholds, unsupported forms, cancellation and exhausted budgets. FR-282-AC-1/2/11 are tagged by real analyzer terminal bytes passed to CARGO_BIN_EXE in a child and altered-value/provision controls; AC-12 by byte/source/storage exact-limit controls, exhaustive cancellation-poll tests and analyzer document stops. The analyzer tests and provider retention test exercise production records; the fresh child links only qualified-core normal dependencies. FR-255 mappings are exhaustively matched by the model-limit setting tests. No new stubs or unowned production behavior were found. Bounded AC-12–14 of FR-201 and AC-13 of FR-282 and the older broad obligations remain future work, not delivered here.

The preallocation source defect is owned solely by code-review FND-001; it is not duplicated in this artifact. Existing budget tests are useful outcome oracles but do not assert early allocation admission. Focused passing logs were inspected, not rerun.

## Assurance Context

No active AssuranceProfile applies: repository search found mentions in historical review artifacts, not a current profile declaration. No active exception was used. No enforced review workflow was required. Full computed matrix and aggregate architecture/string-edge/deny gates were not run under the explicit scoped-review brief; no full assurance completion is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Examined units (clean units included):

```yaml
- id: FR-201-AC-9
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: For an admitted acyclic eventual claim, writing and reading the closed
    profile preserves identity, exact rational values, complete keys, targets and
    required ranks byte for byte. Duplicate/out-of-order rows, unreduced 2/4, denominator
    0, missing ranking, extra fields, padded base64url and a model-only key are rejected.
- id: FR-201-AC-10
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A valid document offered for another identity refuses before enumeration;
    replacing its targets or optimization while retaining valid Bellman rows refuses
    objective mismatch. A same-model key with a different binding is rejected rather
    than merged. Altering a value fails equality; removing or breaking positive-value
    rank progress fails ranking even when equalities hold.
- id: FR-201-AC-11
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Minimum AtLeast, maximum AtMost and resolved-workload exact proofs pass
    the profile checker at an attained threshold; a threshold strictly beyond the
    certified initial value fails. Unbounded cyclic, timed, reward, fair, multi-initial
    and multi-binding cases, bounded forms outside the admitted subset and non-exact
    kinds return explicit unsupported with no accepted certificate. Pre-cancelled
    input and exhausted byte/state/edge/arithmetic budgets stop without a truth value.
- id: FR-282-AC-1
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: 'The EN-5 certificate from FR-281-AC-1, written to bytes by `analyze` and
    passed with the byte provision and the subject binding to the EN-5 checker in
    a fresh process that links only qualified-core crates, is accepted: the process
    re-checks the provision''s sources into a `CheckedPackage`, admits the `ModelSystem`
    and runs the checked-subject form.'
- id: FR-282-AC-2
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: The same certificate with one probability bound changed is rejected with
    a cause naming the failing obligation.
- id: FR-282-AC-11
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: An exact acyclic eventual certificate from a random/workload source package
    that S4 refuses is written, then accepted in a fresh executable linking only qualified-core
    crates after bounded source admission and S1 to S3 checking. No emitted-package
    bytes or fabricated replay request are supplied. The same executable rejects changed
    values, refuses mismatched identity/objective, and preserves typed source/hash
    failures.
- id: FR-282-AC-12
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: With a source provision or certificate document one byte above its caller
    ceiling, the fresh checker stops/refuses before reading beyond that ceiling; pre-cancellation
    and cancellation during graph expansion stop without acceptance or a truth value.
    Exhausted state, edge and arithmetic budgets retain the actual named setting and
    consumption.
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
