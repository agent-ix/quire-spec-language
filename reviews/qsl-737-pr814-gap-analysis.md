---
id: SR-9019
title: "Gap analysis of quire-spec-language PR #814: QSL-737 FND-007 against FR-359-AC-8 and TC-1001 step 2"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c7e80a9d1a36127e4a1107b02594b4d9b0369d11; FR-359-AC-8, FR-359 item 6, TC-1001 steps 2-3, QSL-737 gap-analysis FND-007; qsl-eval/tests/it/dispatch_calls.rs and the production paths it exercises"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: reviews
---
# Gap analysis of quire-spec-language PR #814

## Summary

Ticket: QSL-737. This review measures the two new tests against the open
reviewer finding FND-007 (SR-8694). FND-007 reads: "The `Upcast` of the
operation and a redeclared subtype body are still not exercised at run
level". The review also measures them against FR-359-AC-8, FR-359 item 6 and
TC-1001 step 2.

QSpec FR-151 (`quire.model.dispatch.single/v1` and
`quire.model.conformance.refinement/v1`) supplies the dispatch and guard
semantics. FR-152 and FR-153 add nothing about argument upcasts or the
binding of a top-level receiver.

The trace tags are correct. TC-1001 is scoped to FR-359-AC-8, and step 2 is
this behaviour.

Semantic review (intent, test and code, plus oracle strength) was run by
hand, as the dispatching brief requires.

## Verdict

**CONDITIONAL.** What the PR now covers at run level:

- **The Upcast.** In FR-359 item 6 ("an `Upcast` of an operation parameter")
  and FR-151 ("type-checked statically against `o`'s signature with
  reference upcasts only"), the Upcast is the admission of the argument at
  the dispatch parameter: `Frame::Upcast`, then `upcast`, then
  `Form::Argument`. Both new tests run it with a `TaxedMoney` at `Money`.
  The base test from 14fda4097 already did. That half of FND-007 is covered,
  but not in the way the coder described: the "original root" reading
  (FND-002 of SR-9018) is not what the Upcast is.
- **A subtype's own body and guard over a child argument.** `B.size` gives
  105 and 140; amount 4 gives `PreconditionFalse` with `selected =
  model.B.size`; a plain `Money` gives 107; a `Fee` is refused
  `type-mismatch`.
- **An ancestor body over a child argument through a real bridge-linked
  family with two candidates.**

The guard fixture matches FR-151. The effective precondition is the member's
own clause OR the effective preconditions of the members it redefines, and an
absent precondition is `true`. `A`'s `>= 1000` disjunct is inert in every
vector, so no vector tells "own OR ancestor" apart from "own only". That is
FR-151-AC-4 / TC-196 territory, not AC-8, so it is not a finding here.

Still unexercised: FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers the FR-151 dispatch case of TC-1001 step 2, "through a dispatch guard and body of a subtype that redeclares it". In that case the call site is statically `Reference<A>` (`A.size`'s family), the runtime receiver is a `B`, and `dispatch.select` (`linked_for(B)`) picks `B.size` with effective guard `B` OR `A`, over a `TaxedMoney` argument. The `B`-rooted vector types the receiver as `B`. Its table has `B.size` as its only candidate, as the file's own D06 note says. So the redeclared body is never selected over an ancestor candidate, and "the ancestor body did not run" holds by construction. The PR says a `B` receiver at an `A`-typed receiver parameter is not bindable at top level. That rests on the bridge test's doc, which cites `quire_exact::ValueType::admits`. But `CheckedPackage::evaluate` validates arguments with `TypeEnvironment::admits`, which in QSV ce9a0f1 admits a `Reference` whose type conforms, and `money_bridge`'s environment declares `B` with supertype `A`. The claim looks stale, and that doc should be corrected with the test. Confidence is medium: this is not run (reviewer-only) | qsl-eval/tests/it/dispatch_calls.rs:1239-1245,1999-2013,2930-2954; qsl-eval/src/value/expression/mod.rs:191,651-656; qsl-eval/src/value/expression/evaluate.rs:1777-1798 |

## Coverage

- FR-359-AC-8: `tagged`. Run-level binders now include the two new tests and
  the earlier `a_dispatched_call_with_a_child_argument_runs_through_guard_and_body`.
  Still unbacked: the FR-151 subtype-dispatch case (FND-001), and "naming the
  parameter" on the dispatch refusal (existing; outside this PR's claim).
- TC-1001 step 2: the Upcast is covered, and a redeclared body run directly
  is covered. Selection of the redeclared body by dispatch is not covered.
- Bindings checked: both new tests to FR-359-AC-8 / TC-1001, correct.
- Reverse gap and stubs: the diff is test-only and adds no production code.
  It has no stub and no ignored test.
- Semantic review: run by hand on this scope.
- Plan completion: not assessed

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 0c14fc514: `dispatch_selects_the_redeclared_body_for_a_subtype_receiver_over_a_child_argument` covers the FR-151 case. It uses a `Reference<A>` call site, the `A`-rooted family with both candidates, a runtime `B` receiver bound through `TypeEnvironment::admits`, and a `TaxedMoney` argument. Results: amount 5 gives 105, amount 1000 gives 1100, amount 4 gives `PreconditionFalse` with `selected = model.B.size`, and a `Fee` refuses `type-mismatch`. `dispatch_selects_the_ancestor_body_for_an_ancestor_receiver_over_a_child_argument` covers an `A` receiver at the same site: it selects `A.size` (1000 gives 1000, a `Money` of 2000 gives 2000, and 9 gives `PreconditionFalse` with `selected = model.A.size`). The stale "cannot bind" doc is corrected. Both tests pass in `qsl737u-fix-gate.log`. Residual: the `A` disjunct of the guard is not decided (SR-9018 FND-004) |
