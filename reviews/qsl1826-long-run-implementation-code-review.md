---
id: SR-20261110045048840001
title: QSL-1826 long-run implementation code-review
type: SpecReview
analysis: code-review
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
| FND-003 | medium | Missing values or component evidence members fall through serde to a generic Malformed error instead of the required Rejected(Malformed, Member(values/components)) result. | qsl-replay/src/probability_certificate/document/long_run.rs:134-141 |

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
| FND-003 | fixed | Bounded parsed-node structural admission checks required values and component value/gain/bias before serde; public checker and source facade return Rejected(Malformed, Member(values/components)). Actual public tests assert each member locus and retain an accepted control. |

Latest verdict: **FAIL / BLOCKED solely on FND-001**. No new concrete regression or wrong-acceptance defect found in the scoped fix. Twelve focused tests and four fresh-process tests pass in the actual fix logs, as do scoped all-target Clippy and formatting. These are supplied final-head results inspected, not new reviewer execution. No full matrix or owner-bound qualification is claimed.

```yaml
dispositions:
- fnd: FND-001
  outcome: still-open
  reason: No raw bounded/cancellable owner API adoption landed. The same legacy system.state_key
    and transitive candidate/post-key paths allocate before caller scratch reservation;
    upstream IR142/143 and typed callback adoption remain required. No waiver or merge
    authorization.
- fnd: FND-003
  outcome: fixed
  after_excerpt: "rows(root.get(\"values\"), \"values\", cx)?;\ncx.work(1)?;\n   \
    \     if component.get(\"value\").is_none() || component.get(\"gain\").is_none()\
    \ {\n            return Err(ProbabilityDocumentError::MalformedMember(\"components\"\
    ));\n        }\n        rows(component.get(\"bias\"), \"components\", cx)?;\n\
    \    }"
```

## New findings (disposition pass 2)

Frozen adoption head b0d6d4d1b181ce21b464db4bfd9e28238d5fa163, actual merged owner base 632caf8cebf0ec21f897a1b6e870c5e79e75b531.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The new admission and full-product wrappers inspect Cancel::tripped without polling the token, so cancellation requested during reserve-only parse/staging/output can remain unobserved through construction completion. | qsl-replay/src/probability_certificate/key_admission.rs; qsl-replay/src/probability_certificate/document/key.rs:78-83;92-96 |

At exact dependency quire-exact4fe7df6, Cancel::cancel sets cause while Cancel::tripped only reports a cause previously observed by poll. Reading tripped is therefore not a polling boundary. The helper has one work charge before parsing, followed by reservation-only parsing/staging/output. The callback may request cancellation after that charge, without latching it for either wrapper. Fix both wrappers and use actual reservation-phase controls, rather than poll-count offsets. No numerical proof error was identified.

## Dispositions — round 2

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Raw owner API adoption is present, but complete requalification of FND-001 remains pending the actual full-product cancellation fix and final frozen-head review; no merge authorization is granted. |
| FND-004 | still-open | Both new poll wrappers still need to call Cancel::poll before inspecting the recorded cause and pass phase-driven full-product Requested/Deadline controls. |

```yaml
dispositions:
- fnd: FND-001
  outcome: still-open
  reason: Raw owner API adoption is present, but complete requalification of FND-001
    remains pending the actual full-product cancellation fix and final frozen-head
    review; no merge authorization is granted.
- fnd: FND-004
  outcome: still-open
  reason: Both new poll wrappers still need to call Cancel::poll before inspecting
    the recorded cause and pass phase-driven full-product Requested/Deadline controls.
```

## Final adoption disposition — round 3

Verified frozen 4129789466f2749ceddf00f8795b99b547e34d4c on merged owner base 632caf8cebf0ec21f897a1b6e870c5e79e75b531. Original findings and all prior disposition rows are preserved above.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Merged owner state_key_bytes_with_budget and weighted_steps_with_budget now feed the original shared ProbabilityKeyAdmission in checker subject enumeration and the private optional producer build bridge. The same mutable meter/storage counter continues through current, undecided frame-candidate, post-state, full-product parse/canonical staging/output, values and component keys; identity.input_bytes is separate from certificate byte limits and model integer representation uses IntegerBits. No legacy raw state-key fallback remains on the qualified long-run path. |
| FND-004 | fixed | Both admission and full-product wrappers call the original Cancel::poll before inspecting tripped. The public full-product reservation callback test reaches a real parse/staging/output reservation, then issues Requested/Deadline with both the real admission adapter and a reservation-only provider. Denied retains the exact adapter cause, or the wrapper returns the exact direct owner cause; uncancelled controls equal the legacy complete key. No fixed callback/poll offset is used. |

Latest verdict: **PASS for the scoped long-run implementation**. No remaining PR finding or newly observed concrete regression. FND002/003 remain fixed from round1. Mergeable within the explicit implementation boundary; no full AC30/all-expansion qualification.

Independently reviewed all adoption/new caller code at final frozen 4129789466f2749ceddf00f8795b99b547e34d4c against actual merged owner base 632caf8cebf0ec21f897a1b6e870c5e79e75b531 and the original b08cda324 fix source. The complete PR now changes 20 files including two historical review-custody files; all new producer/checker/private-builder/key-adapter/full-key writer and test changes were inspected. Owner PR2654 was independently reviewed separately; its existing contract usage and final original-SR dispositions were read here, without claiming to replace that owner review. No dependency override/pin, new claim profile, policy, CI or manifest change is introduced in this caller delta. Old ProductRequest literals and legacy builder wrappers remain valid; the optional KeyPolicy borrows one budget, and absence retains the old path. The legacy readings cache changes from a pair key to binding-indexed inner maps without changing semantics.

Owner/adoption: ProbabilityKeyAdmission borrows the same Meter, storage count and original Cancel; recreating a view does not reset counters. Checked addition and reserve-before-growth retain storage/representation/work causes. Exact raw/full identity byte limits and allocator/overflow failures remain distinct from document transport/storage and probability scalar limits. Full-product construction admits the canonical reader, raw canonical re-encoding, complete-key staging and destination copy through the same Reservation. Source and real binding identity remain independently checked. The private producer bridge retains typed product, key, cancellation and undefined failures, and the qualified workload path calls the actual owner budgeted expansion API for internal current, undecided-candidate and post-state identity work. Its large-candidate test uses a genuine undecided postcondition (value10..98, actual acceptedpost99), not a synthetic external key callback or count offset. Both Requested and Deadline retain original causes.

Actual merged-owner evidence inspected: 22 selected long_run cases and four fresh-process cases passed at bb577de1f; final b0d6d4d1 differs only by an unused test import. Thirteen existing finite-builder controls passed; touched three-package all-target Clippy and fmt passed on actual merged owner. Following FND004, final23 selected cases at 412978946 pass in /tmp/qsl1826-long-run-fnd004-tests2.log, with touched three-package all-target Clippy and fmt passing in the same prefix logs. Evidence manifest /tmp/qsl1826-long-run-merged-owner-validation.json gives chronology. No reviewer heavy rerun or full workspace/aggregate qualification. Exact/one-short cumulative raw/full-key storage, repeated-call work/storage, larger-post identity bytes, actual internal undecided candidate cancellation, representation IntegerBits versus rational scalar caps, and full-product reserve-phase cancellation have discriminating public assertions plus uncancelled controls. ModelCheckLimit::IdentityInputBytes exposes the real identity.input_bytes setting; constructor/helper controls inspect actual typed owner limits, and the outcome wire control verifies that spelling. Earlier numerical/malformed/coverage fixes remain byte-preserved in source and passing regressions.

Scope boundary independently classified: ModelKeyBudget explicitly covers raw STATE identity work and excludes ordinary model interpretation and the separate transition-identity encoder. Existing ModelTransition::new/embed private reader allocations (frames.rs:77,441), model/interpreter construction and unrelated transition payloads predate this adoption and are not newly claimed bounded here. They remain a prerequisite to full FR201AC30/all-expansion working-storage assurance. The original FND001 names raw model-state and transitive candidate/post STATE keys; those paths now use the required owner scope and are fixed. This is a scoped implementation PASS, not a waiver of complete expansion budgets and not a declaration that all AC30 obligations or QSL1826 are finished. Full repository computed matrix remains unavailable from the recorded attempted run; no matrix coverage figure, whole-repository assurance PASS, or new full semantic campaign is claimed. Plan completion: not assessed.

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "let bytes = system\n                    .state_key_bytes_with_budget(state,\
    \ identity, budget, cancel)\n                    .map_err(Error::Key)?;\n    \
    \            ProbabilityProductKey::for_model_state_with_budget(\n           \
    \         reference, &bytes, identity, budget, cancel,\n                )\n  \
    \              .map_err(Error::Key)"
- fnd: FND-004
  outcome: fixed
  after_excerpt: "fn poll(&mut self) -> Result<(), ReservationDenied> {\n        self.cancel.poll();\n\
    \        if let Some(cause) = self.cancel.tripped() {\n            return Err(self.deny(ProbabilityDocumentError::Cancelled(cause)));\n\
    \        }\n        Ok(())\n    }\nlet poll = || {\n        cancel.poll();\n \
    \       match cancel.tripped() {\n            Some(cause) => Err(ModelKeyError::Cancelled(cause)),\n\
    \            None => Ok(()),\n        }\n    };"
```
