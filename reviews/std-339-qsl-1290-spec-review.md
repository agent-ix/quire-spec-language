---
id: SR-16198
title: "spec-review of the FR-262-AC-12 and FR-401-AC-8 evaluator scope (quire-spec-language PR 1290)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@44a3c9d3; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md; spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md; spec/test-cases/TC-1124-quire-eval-is-the-only-evaluator-after-the-switch.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1124
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: references
---

# Spec review of the FR-262-AC-12 and FR-401-AC-8 evaluator scope (quire-spec-language PR 1290)

## Summary

Ticket: STD-339. This is one spec-review pass over
`git diff origin/main...44a3c9d3`. The diff touches FR-262-AC-12, FR-401
Behaviour 4, a new FR-401 paragraph on `call` and `evaluate` under a
cancelled meter, FR-401-AC-8, the new FR-401-AC-9, FR-405 item 2 and
FR-405-AC-2, and TC-1124 step 2 and expected result 2. The oracles were the
FR-262 Begin and cancellation text, FR-401 Behaviour 3, FR-277-AC-4, merged
#1280, `qsl-eval` on origin/main (`CheckedPackageEvaluation::call`,
`call_supplied`, `evaluate`), the code PR #1269 at 99b62b56, and the tests
the new criterion cites: `qsl-eval/tests/it/equality_matrix.rs`,
`qsl-eval/src/value/expression/evaluate/invariants.rs` and
`qsl-eval/tests/it/model_reference_queries.rs`. The code-review and
gap-analysis methods did not run because the diff has no code.

Gates: origin/main is an ancestor of the head. `quire validate` over the four
touched files exits 0 (quire 0.36.2, engine 0.50.2).

Examined, clean:

- **Pre-admission keeps every poll index for `call` and `evaluate`.** On
  origin/main both entries already admit through
  `SuppliedRun::new(SuppliedLimits::default(), Cancel::new())`, a handle no
  caller holds, and the meter's handle is first polled at the first meter
  charge. Pre-admission therefore adds no poll of the meter's handle and
  removes none. The population-query test (`model_reference_queries.rs`
  4108-4189) trips the handle at poll `stopped` and asserts
  `prefix[stopped]`. Under pre-admission that poll is still the
  `stopped`-th meter charge, so the test passes unchanged and FR-401-AC-4
  holds.
- **FR-277-AC-4 agrees.** `call` stays `call_supplied` at the default
  ceilings over a handle nobody cancels.
- **The population-query vectors of FR-401-AC-9 match the suite.** For
  both `call` and `evaluate`, a handle cancelled before the call gives
  `Ok(Incomplete)`, no admitted charge and `tripped()` equal to the cause.
  The n-th-poll loop asserts the stop point and the admitted prefix.
- **The FR-405 and TC-1124 additions agree.** FR-405 item 2, FR-405-AC-2,
  TC-1124 step 2 and expected result 2 all require FR-401-AC-8 in full,
  including `g(3)`, on `quire-eval`'s evaluator on the switch base. FR-405
  already runs each implementation there.
- **FR-401-AC-8's vectors are unchanged.** It now binds the steppable
  evaluator. Through the std reference evaluator it binds the `f(3)` run and
  the requesting-body run, and it leaves out `g(3)`. The criterion states
  which evaluator each run binds, with no status wording.
- **Ids are unique.** FR-401-AC-9 appears in no other open QSL PR diff. On
  main it appears only in `reviews/std-335-spec-review.md`, as #1280's
  removed draft.
- **The lead defaults are labelled.** The PR body marks every point as a
  lead default, not an owner answer.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The new FR-401 paragraph says `call` and `evaluate` "begin the session over arguments already admitted, so Begin's admission polls no handle". FR-262 defines no such Begin. Its Begin text says Begin's argument admission "SHALL take the session's meter" and "SHALL poll the meter's installed handle at FR-277's poll points", and that an already cancelled handle ends Begin with the pre-charge `CallFailure::Cancelled`. FR-401 Behaviour 3 says the contract's call and expression admission take the meter "so the implementation's supplied-value run polls the meter's installed handle". `call` and `evaluate` reach `quire-eval`'s Begin with a meter whose installed handle is cancelled, so these unconditional SHALLs require `Cancelled`, and the new paragraph requires `Ok(Incomplete)`. #1269 does this through `PackageClosure::pre_admitted`, whose contract admission polls nothing. That mode is unspecified. Fix: state the pre-admitted Begin, or the closure whose admission is already done, in FR-262's Begin text and in FR-401 Behaviour 3, and say which callers use it. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:186-190, :225-239; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:150-170 | wrong-requirement |
| FND-002 | high | FR-262-AC-12 now binds `qsl-eval`'s `call_supplied`, but #1269's `call_supplied` (99b62b56, `qsl-eval/src/value/expression/mod.rs` 921-958) admits under the caller's run alone and then begins over `PackageClosure::pre_admitted`. Its admission never polls the meter's handle. So two of AC-12's runs fail through it: the meter's handle tripping at the second argument's first poll point, and the caller's run uncancelled with the meter's handle cancelled. Both give `Ok(Incomplete)` at `function.call`, not `Cancelled`. AC-12's vectors are also written for Begin ("Begin of a checked expression", "no session", "return a session"), and `call_supplied` has neither an expression form nor a session, so it is unclear which runs bind it. Fix: either bind AC-12 to `quire-eval`'s Begin alone, or keep `call_supplied` and require #1269 to route its admission through Begin with the caller's run and the meter. In that case, list the four runs over `call_supplied` (call form only, result in place of session). | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:289; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:236-238 | wrong-requirement |
| FND-003 | medium | FR-401-AC-9 says that over the TC-918 `Compare` package, `call` and `evaluate` under a cancelled meter "each return `Ok` ... `Incomplete`". The suite's TC-918 cancellation vectors are `call` only (`equality_matrix.rs` 374-386, `invariants.rs` 1492-1520), and they run inside the faulting variant. In that variant `evaluate` makes no meter charge before the S6a fault: the expected prefix is `[]` (`equality_matrix.rs` 346-363, `invariants.rs` 1469-1474). A cancelled meter therefore gives `Err(CallFailure::Fault)` there, as the new FR-401 paragraph's "an evaluation that makes no meter charge completes" implies. The AC asserts a result that is false for that run, and no existing test asserts it. The population-query run cancelled before the call also asserts no charge point (`model_reference_queries.rs` 4131-4139), while the AC states `ChargePoint::FunctionCall` for `call`. Fix: limit the `Compare` vectors to `call` (charge point `FunctionCall`), and for `evaluate` name the population query only, or state the fault result. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:289 | wrong-requirement |
| FND-004 | medium | FR-401 Behaviour 4 keeps its unconditional SHALL: a registered call site dispatches to its host body "wherever the call site is", including "a source application inside an interpreted body". It then says "Every evaluator that implements this contract's interpreted transition returns each source application to the scheduler". The std reference adapter implements the contract (FR-401 :213-214), but it runs an interpreted frame to its terminal result and runs the callee's interpreted body at an inner call site. As written, the statement is false of a conforming implementation, and the SHALL is violated by the evaluator this FR requires. Fix: scope the SHALL to evaluators whose interpreted transition returns each source application as a child request. State the reference evaluator's narrower dispatch (root application and child request) as its own SHALL. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:193-208 | wrong-requirement |
| FND-005 | low | Three phrases describe change rather than behaviour. "`call` and `evaluate` keep the result they return" and "admit their arguments as they did before the interface" are framed against history. "FR-405's switch replaces it with an evaluator" is sequencing inside a behaviour item. Fix: state the admission directly ("through a supplied-value run of their own over a handle no caller holds") and drop "keep" and "as they did before". Leave the replacement to FR-405. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:207-208, :225-227 | wrong-requirement |

## Verdict

Not mergeable yet. The central claim holds: pre-admission over a handle no
caller holds keeps every poll index, because origin/main already admits that
way. The population-query vectors, FR-405, TC-1124, AC-8's evaluator scope
and the new id are sound.

- FND-001: the pre-admitted Begin that `call` and `evaluate` rely on
  contradicts FR-262's Begin SHALLs and FR-401 Behaviour 3.
- FND-002: binding AC-12 to `call_supplied` contradicts #1269's
  `call_supplied`, so the PR does not unblock #1269 as written.
- FND-003: AC-9's `Compare`-over-`evaluate` vector is false for the faulting
  variant and is in no test.
- FND-004: Behaviour 4's SHALL still binds the reference evaluator that the
  same item exempts.

FND-001 and FND-002 need a decision: either AC-12 binds only Begin, or #1269
changes `call_supplied`. FND-003 to FND-005 are wording fixes.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At d3b96b93, FR-262 defines the pre-admitted Begin: the caller admits under its own run, and Begin admits nothing, polls no handle and returns no admission failure. FR-401 Behaviour 3 says a closure built over already-admitted arguments admits nothing at Begin. `call` and `evaluate` begin through it. |
| FND-002 | fixed | FR-262-AC-12 binds only `quire-eval`'s Begin with contract admission. `call_supplied` admits under the caller's run alone and then uses the pre-admitted Begin, which matches #1269 at 99b62b56 (lead default for `call_supplied`). |
| FND-003 | fixed | The `Compare` vector in FR-401-AC-9 is `call` only, at `ChargePoint::FunctionCall`. The population-query vectors cover `call` and `evaluate`, and the n-th-poll vector stops at the n-th charge of the suite's prefix. This matches `equality_matrix.rs`, `invariants.rs` and `model_reference_queries.rs`. |
| FND-004 | fixed | Behaviour 4 has two scoped SHALLs: the steppable evaluator dispatches at every call site, and the std reference evaluator dispatches at the root application and at a child request. |
| FND-005 | fixed | "keep the result", "as they did before the interface" and "FR-405's switch replaces it" are gone. |
