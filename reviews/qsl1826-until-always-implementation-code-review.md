---
id: SR-20261109172912000057
title: "Code review of acyclic until and always certificates"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1826 PR2625; qsl-analyze/src/analyze/exact.rs, qsl-analyze/src/exact_probabilistic/unbounded.rs, qsl-replay/src/probability_certificate{.rs,/document.rs,/document/writer.rs,/state_predicate.rs}, qsl-replay/src/lib.rs, changed integration tests and qsl-probability-check/README.md"
review_set: subset
---

## Summary

Independent code/Rust review of frozen 65363e360fb3054ca4adf3a3b64a4dc2926d9f45 against b9eacce294c549b1e9e36984fe8470b52ed6a5e1, QSL-1826 PR2625. No actionable defect found in the scoped implementation.

## Verdict

**PASS** for the implementation scope. Original graph admission precedes stopped-graph construction; the checker derives event, labels, optimization and complete binding-bearing keys from checked source. Until gives B precedence, absorbs neither-A-nor-B, and rejects unresolved terminals; always continues through nonterminal P and accepts terminal P under infinite stutter. Bellman, complete coverage, rank progress and thresholds remain independently checked. Native state-predicate refutations explicitly remain unsupported.

## Assurance Context

No selected AssuranceProfile found in the repository guidance or dispatch. AGENTS.md separates engineering delivery from the wider assurance campaign. Existing focused logs were accepted as authorized evidence; no aggregate, full matrix, Kani or make ci run was performed. Cargo deny was not rerun: no dependency, manifest, lockfile or CI change. Active exceptions: scoped checks and manual gap review authorized by the lead; no other exception identified. This is not repository-wide qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review evidence

Reviewed the Rust API, typed decoding, caller-controlled allocation, cancellation, boundary and test requirements against rust-style and repository AGENTS.md/CLAUDE.md. New wire members use the existing borrowed writer reservation path; failure keys are admitted before transport growth. Reader requires matching profile/event/failures presence, strict key sets, no overlap, canonical bytes and shared rational accounting. The model enumeration uses the caller key limits/cancellation, counts queued and processed identities, and checks original acyclicity before absorbing any boundary. Source admission and full binding remain checker-owned; no solver linkage is introduced into the standalone executable.

The qualitative solver fix preserves decided non-target absorbing states in safe sets and prevents attractor traversal across a reached target. Reviewed both new owner regressions plus the real source fixture that previously gave the wrong minimum; no checker weakening accompanies the fix. Existing generic reachability, fair/cyclic and min/max owner cases remain green in the focused 39-test log.

Accepted logs: /tmp/qsl1826-until-tests-final.log (25 fresh-process tests), /tmp/qsl1826-until-owner-tests.log (39 engine tests), /tmp/qsl1826-until-adapter-tests-final.log (13 adapter tests), /tmp/qsl1826-until-core-tests.log (17 core tests), /tmp/qsl1826-until-clippy-final.log (focused all-target clippy), /tmp/qsl1826-until-fmt-final.log (fmt). No source edits or test reruns were necessary. CI/workflow/manifest/lock diff is empty.

## Examined units

The following excerpts identify scope; each AC was read in full, not limited to its excerpt.

```yaml
scope:
  - id: FR-201-AC-15
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'A reached B-and-not-A state has value 1; initial B accepts at 1, initial neither-A-nor-B at 0, and terminal A-and-not-B at 0.'
  - id: FR-201-AC-16
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'A prefix not-P has value 0 despite later P; initial terminal P has value 1 and initial not-P value 0.'
  - id: FR-201-AC-17
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'Changed event/optimization/targets/failures refuses objective mismatch; a foreign S3 binding or altered complete key is not an alias.'
  - id: FR-201-AC-18
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'No state-predicate proof document is treated as a refutation witness.'
  - id: FR-282-AC-14
    path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
    role: examined
    excerpt: 'Existing reachability/v1 bytes and verdicts are preserved.'
  - id: FR-282-AC-15
    path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
    role: examined
    excerpt: 'No analyzer/provider linkage, emitted-package bytes, fabricated replay request, solver labels, local state ids or new witness wire are used.'
  - id: FR-201
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'The checker SHALL solve no equations and reuse no EN-5 graph, labels or complement result as trusted evidence.'
  - id: FR-196
    path: spec/functional/FR-196-build-the-probabilistic-product.md
    role: context_only
    excerpt: 'Build the probabilistic product'
  - id: FR-198
    path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
    role: context_only
    excerpt: 'Decide unbounded reachability and expected rewards'
  - id: FR-126
    path: spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md
    role: context_only
    excerpt: 'Check a temporal clause over every behaviour of a model'
  - id: ADR-028
    path: spec/decisions/ADR-028-exact-probabilistic-engine.md
    role: context_only
    excerpt: 'Exact probabilistic engine'
  - id: ADR-018
    path: spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md
    role: context_only
    excerpt: 'Temporal properties over every behaviour'
```
