---
id: SR-4934
title: base review of QSL-682 evaluation-core interface
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language@b54c3a45f3bde2abb8675d01ec10bfde3eabfadd; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md;
  QSL-682
review_set: subset
---

## Summary

All nine criteria and added operational statements were examined. IDs/links and method-only verification cells are coherent. AC3-9 are deliberately future implementation obligations. Manual counterexamples distinguish replay/recharge, wrong child continuation, terminal resumption, cancellation, and std-only migration. Jev automated criterion-strength is unavailable; no automated strength verdict is claimed.

## Verdict

PASS — specification-only review; no runtime/full-gate acceptance.

## Examined Scope

```yaml
scope:
- id: FR-262-AC-1
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: 'On a thread with a 512 KiB stack, the recursive function `function count
    using v(n: Int[0, 1000000]): Integer pure decreases(n) { if n = 0 then 0 else
    count(n - 1) + 1 }`, called with `100000` under a `work_units` budget sized for
    it, completes with `100000`. With `work_units` one below the run''s measured spend,
    the same call returns `incomplete { limit_kind: work_units }` at the denied charge,
    and completes once `work_units` is raised to the measured spend.'
- id: FR-262-AC-2
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: 'On a thread with a 512 KiB stack, a recursive list value 100,000 long
    (`record List { head: Int[0, 9]; tail: List?; }`), built by a recursive function,
    evaluates; it keys as simulation state, and its state-key bytes equal the RFC
    8785 text of its canonical JSON form; it compares equal to its clone, and it and
    its clone drop. A `ValueType` of 100,000 nested `Option`s around `Boolean` clones,
    compares equal to its clone, hashes equal to its clone, formats for debug and
    drops on the same thread.'
- id: FR-262-AC-3
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: Inspection finds one QSL-owned core interface without checker/package dependencies
    or RT/CG access to evaluator internals; registration cannot create checked authority
    or substitute another closure.
- id: FR-262-AC-4
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: A generated body calls a second generated body that calls an interpreted
    function. Stepping between every handoff and return produces the reference result
    and charge sequence, resumes each caller once, preserves locals and call-site
    locations, and never recursively invokes the evaluator.
- id: FR-262-AC-5
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: A source call with two argument applications charges them left to right
    before its own call charge and body work. Steps add no charge; a standalone literal
    charges no `function.call`; rejected root inputs cause no charge or body invocation.
- id: FR-262-AC-6
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: For AC-4's mixed-body chain let `w` be reference work spend. Limit `w`
    completes; `w - 1` returns the reference located incomplete result and denied
    charge, keeps earlier spend and leaves the denied charge unrecorded. No subsequent
    step can run a suspended body; a fresh invocation at `w` completes with static
    totality unchanged.
- id: FR-262-AC-7
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: Cancelling after a child request is saved, before the next transition,
    returns `CallFailure::Cancelled` with no further charge or body execution. In
    a direct-return caller, a child refusal or undefined outcome propagates with reference
    location/loss behavior and runs no subsequent body work; dropping the session
    executes no continuation.
- id: FR-262-AC-8
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: A body performs charged work, saves a local, requests a child and continues
    with its returned value. Each body transition and child occurs exactly once across
    steps; prepared-entry execution matches the synchronous reference, including a
    kernel loss record, and unsupported preparation stays a pre-application disposition.
- id: FR-262-AC-9
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: An interface crate with only a std reference adapter does not satisfy RT's
    complete evaluator migration. That migration consumes the shared implementation
    with std disabled on RT's governed no_std target, supplies checked-input and generated-body
    evaluation through this interface, and leaves no RT-local evaluator copy or compatibility
    re-export.
- id: FR-262
  path: spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md
  role: examined
  excerpt: The QSL-owned shared evaluation-core interface SHALL own resumable application
    state, the scheduler step and execution fuel for interpreted checked expressions
    and registered generated host bodies.
- id: QSpec-FR-146
  path: /home/peter/dev/quire-specification/spec/functional/expressions/FR-146-check-total-pure-functions.md
  role: context_only
  excerpt: Each call charges `function.call` under `quire.value.accounting/v1` before
    its arguments are bound. Arguments are evaluated left to right before that charge.
- id: FR-276
  path: spec/functional/FR-276-cancel-a-lifecycle-operation.md
  role: context_only
  excerpt: When the handle is cancelled, each lifecycle operation shall stop at its
    next charge and return its cancelled failure with the handle's `CancelCause`.
- id: FR-294
  path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md
  role: context_only
  excerpt: 'One trait in layer 5 (`qsl-eval`, module `execution`) holds every execution
    backend (ADR-029 EB-1):'
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and Limits

Direct changed-file Quire validation passed (1/1 grammar-clean; 4/9 criteria property-extractable). Installed quire 0.36.2 engine 0.50.2, quoin 0.28.3; module spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502. Module discovery emitted DuplicateArchetype ADR/Plan/Review/SpecReview/Standard, DuplicateInverseEdge part_of and semantic.inline-data-schema advisories, retained in changed-validation.log. No applicable AssuranceProfile found. Native code builds/tests and enforced workflow were outside this spec-only scope. Automated Jev is unavailable; manual AC falsifiability only. Available reviewer model metadata is GPT-6 (no more specific model identifier exposed); no guessed version.

Computed matrix completed (metadata only, exit 0): FR262 AC1/2/4/5/6/7/8 untagged; AC3/9 method-without-symbol. AC1/2 gaps are inherited; AC3-9 are explicitly future downstream obligations. No full-coverage claim. RT PR111 FR273 at 19ce9e7f3e7c206fe8ca1bf555fb742576ddcaa2 was read directly and agrees on shared meter, order and explicit frames.
