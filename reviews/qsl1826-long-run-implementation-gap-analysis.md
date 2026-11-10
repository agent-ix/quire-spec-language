---
id: SR-20261110045048840002
title: QSL-1826 long-run implementation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2648; 11-file runtime delta; FR201
  AC27-30 and FR199 AC1
review_set: subset
---

## Summary

Independent blocked-draft runtime review of frozen 21f1c51e5da3135d10f8c117f00e49b6c9d27dce, base 971eb4f36a1f3ea05b4f98559894ed470806c443, PR2648. Ticket QSL-1826.

## Verdict

**FAIL / BLOCKED** — required raw owner admission remains missing; additional focused coverage and member diagnostics need correction in the same PR. No merge/runtime qualification is granted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Long-run graph and transitive candidate/post-state keys are constructed before caller scratch reservation through legacy raw identity APIs, so required bounded/cancellable owner admission is not adopted. | qsl-replay/src/probability_certificate/document/long_run.rs:472-494 |
| FND-002 | medium | Long-run-specific AC29 adverse-input coverage omits malformed/noncanonical encodings, duplicate/out-of-order rows, foreign/model-only binding keys, predicate mismatch and early identity conflict; legacy codec controls do not exercise the new decoder. | qsl-analyze/tests/it/long_run_certificate.rs:95-209,385-396 |

## Assurance Context

No applicable active AssuranceProfile found in local spec scope. No active exception used. Direct delegated review; no enforced workflow acceptance claimed.

## Coverage

Frozen PR2648 reviewed against exact merged spec/correction base 971eb4f36a1f3ea05b4f98559894ed470806c443; all 11 changed files, caller admission paths and FR201 AC27–30/FR199AC1 inspected. No spec/plan, dependency, Cargo.lock, CI or gate delta; no spec-review submethod applies. Reviewer, code-review/Rust lane, rust-style API/implementation/safety/wire/testing references, gap-analysis, repository AGENTS/CLAUDE/README and installed SpecReview skeleton read. No applicable active AssuranceProfile or exception found. No production edits or new heavy build were made.

Math: original graph expands through predicate-true states; terminal empty choices receive implicit self-support and unchanged indicator. Independent iterative Kosaraju derives every SCC/BSCC; offered bias-key sets are disjoint and compared to the canonical minimum-key label of every independently derived BSCC. Exact full values cover the reachable graph. Gain equals component fraction, first bias is zero, and every original indicator Poisson row is checked without a stationary solver. Complete BSCC fixation makes transient harmonic extension unique, including periodic classes and transient cycles. Final inclusive threshold checks use actual bound. No producer SCC numbers, graph, labels or matrix algorithm authorize acceptance. Zero support edges are filtered by the existing enumerator. No additional wrong-acceptance defect was found.

Wire/API: separate typed LongRunProbabilityDocument preserves existing exact-document literals and event bytes. Closed serde fields, exact normalized signed bias rationals, decoded complete-key ordering, full binding-bearing keys, checked predicate/workload and exact structural claim identity remain required. Source facade dispatches identity admission before compiler/enumeration, and the child checker has no analyzer/provider dependency. Undefined predicate causes remain typed; unsupported weighted/every/fair/timed/finite/multi-binding forms grant no certificate. Missing evidence diagnostic defect is FND-003.

FND-001 is a concrete retained merge blocker, not a waiver: long_run.rs calls system.state_key before reserve; ModelSystem::state_key uses the legacy default raw identity limit (16 MiB), distinct from document bytes/storage. ModelSystem weighted expansion also constructs canonical candidate/post-state keys at frames.rs:1547 and probabilistic.rs:655,698,808 before the external callback. The full-product wrapper currently receives transport ProbabilityDocumentIoLimits. Upstream owner spec142/code143 and actual bounded raw/external/internal callback adoption with typed work/storage/representation/cancel causes are required. The new graph/context reservations, producer row estimates, fallible sort and wrapper-only stop tests cannot bound those prior raw allocations.

Evidence inspected, not rerun: validation manifest records final9 focused tests, fresh-process source roundtrip1 (valid path before the later diagnostic-only branch), legacy codec10, weighted solver control1, scoped three-package all-target Clippy and fmt passing. Logs under /tmp/qsl1826-long-run-{final9-tests,process,legacy-codec,weighted-control,final-clippy,final-fmt}.log checked. Counterfactual removal of gain equality made the intended mutation1 assertion fail; restored source was byte-exact before subsequent diagnostic changes. This does not qualify raw owner bounds. No full crate/workspace suite, cargo-deny, Kani, aggregate CI or full qualification claim; explicit thermal/scoped dispatch supersedes broad skill gate defaults.

Computed matrix attempt `quire matrix --scope <worktree> --format json` emitted duplicate-module diagnostics but no JSON after several minutes and was interrupted. Full repository matrix/traceability qualification is unavailable, not PASS. Manual scoped trace/AC inspection is separate: actual #[trace] binds new tests to TC1401/AC27–30, but FND-002 records missing adverse assertions. No hand-written matrix or coverage percentage. Optional repository-wide semantic campaign was not separately invoked; scoped code soundness and test-intent checks were required by this dispatch. Plan completion: not assessed.

## Examined scope

```yaml
scope:
- id: FR-201-AC-27
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Under Steady, the original up/down chain with up-to-up 1999/2000 and down-to-up
    9/10 accepts a long-run document with one component value and gain 1800/1801,
    bias 0 at up and -2000/1801 at down (choose the fixture's canonical key order
    up before down), and both state values 1800/1801. Both workload bound sides accept
    at equality; AtLeast 1801/1802 and AtMost 999/1000 reject ThresholdMissed. These
    comparison thresholds lie strictly between 0 and 1 under the existing S3 admission
    policy. Changing the bias, setting transported gain to 0, or replacing long-run
    evidence by reachability values preve
- id: FR-201-AC-28
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A transient self-cycling s goes to s with 1/2, P-terminal g with 1/6 and
    not-P-terminal f with 1/3. Independent terminal stutter gives BSCCs {g} and {f},
    respective component value/gain 1 and 0 and bias 0; values s=1/3,g=1,f=0 accept
    both workload bounds at 1/3. A two-state deterministic cycle alternates P and
    not-P and accepts rho=gain=1/2 with normalized biases 0,-1/2 in that key order,
    despite periodicity. Merging/splitting/omitting a BSCC, adding a transient bias
    member or changing transient V rejects; harmonic values alone cannot certify a
    wrong recurrent value.
- id: FR-201-AC-29
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-27/28 documents round-trip canonically through a fresh checker process
    with no analyzer/provider dependency. Noncanonical rational/JSON/key encodings,
    duplicate or incomplete rows/components, foreign bindings, model-only keys, wrong
    normalization or missing components prevent acceptance. Identity conflict refuses
    before enumeration; a changed derived predicate set/optimization refuses objective
    mismatch. Equivalent source changes that preserve every checked condition may
    accept. Earlier profile bytes and verdicts remain unchanged.
- id: FR-201-AC-30
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Pre-cancellation and exhaustion/cancellation during admission, full-key
    creation, workload/predicate evaluation, SCC/membership passes and exact sums
    retain typed nonacceptance without truth values under the actual caller control.
    Reachable undefined P/workload evaluation is never relabelled false/zero. Weighted
    claims emit no document of this profile; the separate existing FR-199 weighted
    zero-weight control retains Unsupported(ZeroWeightComponent). Unresolved workload,
    every-scheduler/MDP, fairness, clocks, multi-post, multiple initials/bindings
    and unsupported kind/profile remain explicitly
- id: FR-199-AC-1
  path: spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md
  role: examined
  excerpt: '`LongRun` under `Steady` has one BSCC `{up, down}` with `π = (1800/1801,
    1/1801)` and value `1800/1801`; its gain–bias pair is gain `1800/1801` at both
    states, bias 0 at `up` and `−2000/1801` at `down`.'
```

## Dispositions

### Round 1

Independently inspected frozen fix b08cda324407bfc48cc5681cb06491fb06aec4a8 relative to original reviewed head. Original findings above remain unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | No raw bounded/cancellable owner API adoption landed. The same legacy system.state_key and transitive candidate/post-key paths allocate before caller scratch reservation; upstream IR142/143 and typed callback adoption remain required. No waiver or merge authorization. |
| FND-002 | fixed | New public decoder/checker and fresh-process controls assert exact noncanonical/malformed refusals, duplicate/out-of-order value/bias/component member rejections, foreign/absent/stale model-only key nonaliasing, predicate mismatch and identity refusal before key enumeration or missing source provision; valid controls must still accept. |

Latest verdict: **FAIL / BLOCKED solely on FND-001**. No new concrete regression or wrong-acceptance defect found in the scoped fix. Twelve focused tests and four fresh-process tests pass in the actual fix logs, as do scoped all-target Clippy and formatting. These are supplied final-head results inspected, not new reviewer execution. No full matrix or owner-bound qualification is claimed.

```yaml
dispositions:
- fnd: FND-001
  outcome: still-open
  reason: No raw bounded/cancellable owner API adoption landed. The same legacy system.state_key
    and transitive candidate/post-key paths allocate before caller scratch reservation;
    upstream IR142/143 and typed callback adoption remain required. No waiver or merge
    authorization.
- fnd: FND-002
  outcome: fixed
  after_excerpt: "\n        check(&foreign),\n        Err(ProbabilitySourceError::Document(\n\
    \            ProbabilityDocumentError::IdentityMismatch\n        ))\n    ));\n\
    \    assert!(\n        matches!(check(&bytes), Err(ProbabilitySourceError::SourceMismatch)),\n\
    \        \"matching identity must reach the missing provision con\n {\n    let\
    \ original = proof_request(LONG_RUN);\n    for member in [\"values\", \"bias\"\
    , \"gain\", \"value\"] {\n        let mut request = original.clone();\n      \
    \  change_certificate(&mut request, |document| {\n            if member == \"\
    values\" {\n                documen"
```
