---
id: SR-16160
title: "Spec review — checked hybrid model and unverified solver trace (FR-447, FR-250)"
type: SpecReview
analysis: base
date: "2026-10-22"
scope: "agent-ix/quire-spec-language; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md, spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md, spec/test-cases/TC-1187-a-hybrid-declaration-checks-to-a-classified-model-and-routes-its-claim.md, spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md at b8debe4b1"
review_set: subset
evaluated_revision: "b8debe4b118561ac2fbafd46bd5a923eb1cccb7e"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1187
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-705
    type: reviews
---

## Summary

Ticket: STD-287 (unblocks QSL-900). PR quire-spec-language#1279 adds FR-447 (S2
`HybridForm`; S3 checks for modes, transitions, continuous variables, rates,
invariants, guards, resets and expression forms; the clock line and the
`rate-one-hybrid` refusal; `CheckedHybridModel` and `CheckedHybridClaim` on the
checked package; the provider request; the S4 emission refusal) and TC-1187. It
also amends FR-250 (classification read from the checked model, the
`Option<HybridTrace>` outcomes, `SolverInconclusive{trace}`, trace
correspondence refusals, FR-250-AC-4) and TC-705 step 4. One SpecReview covers
the PR. Reviewed `git diff origin/main...HEAD` at b8debe4b1 against QSpec #384
(head f758128b: FR-193, FR-331, schema), QSpec FR-272 causes, the shared
grammar's `hybrid-decl` and `temporal-clause` productions, QSL FR-185 (S4
precedent), FR-235, FR-290 and ADR-026 HY-1 to HY-3.

Consistent and clean: every cause FR-447 cites exists in QSpec FR-272
(`conflicting_declaration`/`duplicate-declaration`,
`missing_declaration`/`missing-name`, `invalid_model_binding`/`malformed-declaration`,
`ill_typed`/`type-mismatch`/`non-boolean-root`,
`unsupported_construct`/`expression-form`/`declaration-form`/`not-yet-implemented`,
`invalid_timed_model`/`rate-one-hybrid`). The S4 refusal matches FR-185's
probabilistic precedent and QSpec FR-331 "In-process providers". Classifying
anything but a literal 1 as `Hybrid` is the conservative direction: such a
model goes to a solver, never to EN-6. The `rate-one-hybrid` refusal matches
QSpec FR-193-AC-7 (with the wording edge in FND-005). Resets read over pre-jump
values agree with QSpec FR-193's reachability contract. The trace refusals in
FR-250 match QSpec FR-193 "Hybrid trace" one for one, and a refused trace
refuses the result, as QSpec lead default 3 states. FR-250-AC-4 is concrete
and testable. The eight lead defaults are flagged in the PR body as "not owner
answers", and defaults 3, 4 and 7 are posed as owner questions. FR-447, TC-1187
and FR-250-AC-4 are absent from origin/main and from all 24 open QSL PR heads.
`make check-spec-validation` with QSPEC_DIR at QSpec #384 exits 0; its only
diagnostic on the touched files is FND-010. The head contains origin/main
(fb959d04).

## Verdict

NOT MERGEABLE as written. One high finding: the claim horizon rule undercounts
nested temporal operators, so it misstates the scope of a bounded-solver proof.
Three medium findings: the use-case and AC-3 claim source does not parse
(no `holds(...)`); `HybridFormula` and the admitted temporal operators are
undefined; and FR-290 and FR-235 still read `solver-inconclusive` as having no
payload. The six low findings are wording, coverage and graph fixes. All ten
can be fixed in this PR. Merge order stays QSpec #384 first.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The claim horizon is defined as the greatest upper bound of the claim's intervals. That is wrong for nested operators: `always[0,10] eventually[0,5] holds(p)` reads states up to 15 s, but the rule gives 10, and `until`/`release` nest the same way. The horizon sets the request extent and is the scope a `bounded-solver{horizon}` proof claims (QSpec FR-193: the proof holds within that time horizon), so a provider bounded by it proves a nested claim over too short a run. Define the horizon as the formula's reach, for example the sum of upper bounds along the deepest nesting path, and add a nested case to FR-447-AC-3. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:235-237 |
| FND-002 | medium | The use-case claim `always [0, 10] h.v < 30` and AC-3's `always h.v < 30` do not parse. In the shared grammar a temporal atom is a `temporal-primary` (`true`, `false`, a parenthesised `temporal-expr` or `holds(expr)`), so the atom must be `holds(h.v < 30)`. An operator with no interval is also admitted only under `quire.temporal.infinite-trace/v1`, and the spec does not say which profile `p` selects. As written, AC-3 and TC-1187 step 3 fail at S1 before any S3 rule is reached. Write `always[0,10] holds(h.v < 30)` and `always holds(h.v < 30)` and name the profile the unbounded variant needs. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:280 |
| FND-003 | medium | `CheckedHybridClaim::formula: HybridFormula` is defined nowhere in QSL or QSpec. The Hybrid claims section says only that atoms lower to `HybridTerm`. It does not say which temporal operators a hybrid claim admits (`until`, `release`, the past operators `once`, `historically`, `since`, `triggered`) or what refuses the rest. Two implementers would build different types and admit different claims. Give `HybridFormula` its variants and a refusal (for example `unsupported_construct`/`expression-form`) for any operator outside them. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:130-134 |
| FND-004 | medium | FR-250 gives `SolverInconclusive` a `trace` payload, and QSpec #384 lets the wire `solver-inconclusive` cause carry `trace`. FR-290's plugin cause table on main still reads `solver-inconclusive` as a variant with "no payload member" (FR-290 line 169), and FR-235's `InconclusiveCause` listing still names it as a bare variant. A plugin result entry the wire schema admits would then be refused, or have its trace dropped, by QSL's plugin reader. Amend FR-290's row (read `trace` per QSpec FR-193, or refuse it from a plugin, and say which) and FR-235's listing in this PR. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:109-113 |
| FND-005 | low | There are three wordings of the clock line. FR-447 says "a numeric literal of exact value 1", FR-250 says "the literal 1", and QSpec FR-193 and FR-193-AC-7 say "rate is 1". Under FR-447, `t' = 2 - 1` or `t' = 1/1` in every mode is `Hybrid`, so QSL does not refuse `rate-one-hybrid` where QSpec FR-193-AC-7 says the declaration refuses. The direction is sound (the model goes to a solver), but the two repos disagree on a conformance edge. Use one wording in FR-250 (cite FR-447) and either have QSpec say literal or have QSL fold constants. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:186-188 |
| FND-006 | low | FR-447 has a `depends_on` edge to FR-250, and this PR adds a `depends_on` edge from FR-250 to FR-447, which makes a cycle. FR-447 builds the model and FR-250 maps results over it, so FR-447 → FR-250 should be `references`, or be dropped. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:16-17 |
| FND-007 | low | AC-2 covers each refusal except `ill_typed`/`type-mismatch` (a non-numeric rate, reset value or arithmetic operand, for example `v' = true`) and a non-Boolean guard. Both are SHALL clauses in FR-447 with no AC row, so TC-1187 does not exercise them. Add `v' = true;` and `when { v + 1 }` to AC-2. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:279 |
| FND-008 | low | FR-447 says two packages that differ in a mode, rate or claim have different "package identities". AC-5 then has S4 refuse to emit any package that holds a hybrid model, so there is no v2 package identity to compare. The intended value is the checked package identity (FR-003, FR-259), and the clause does not say how the new records enter that identity's preimage. Say "checked package identity" in the clause, AC-4 and TC-1187 step 4, and state the preimage form. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:241-244 |
| FND-009 | low | Lead default 3 makes the initial states all valuations that satisfy the first mode's invariant. In the use case, `Moving`'s invariant `v >= 0` leaves `v`, `a` and `t` unbounded, so `v = 40` is an initial state and `SlowEnough` is false. The worked example cannot be proved by any solver. Either constrain the example's invariant (for example `v >= 0 and v <= 20 and a >= 1 and t = 0`, though `t = 0` then conflicts with the flow) or settle the owner question on an `initial` clause before QSL-900 builds on the default. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:160-162 |
| FND-010 | low | The Description holds three SHALLs in one statement (S2 builds, S3 checks, the writer lowers). quire flags it `ears:non-singular`, the only validator diagnostic on the touched files. Split it into one statement per stage, or make the description non-normative. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:33-38 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Horizon is now the formula's reach: r(Always/Eventually) = u + r(f), Until/Release = u + max; AC-3 adds the nested case (horizon 15) and an until case (4). |
| FND-002 | fixed | Use case and AC-3 now write holds(...); the use case states p selects quire.temporal.infinite-trace/v1 for the interval-free variant. |
| FND-003 | fixed | HybridFormula and HybridInterval are defined; the future operators are admitted and the past operators are refused expression-form. |
| FND-004 | fixed | FR-290 gains a solver-inconclusive{trace} row (trace read on hybrid claims, Malformed at /cause/trace otherwise); FR-235 lists SolverInconclusive { trace }; FR-250-AC-4 and TC-705 step 4 test the plugin path. |
| FND-005 | fixed | FR-447 uses one wording (numeric literal `1`, no constant folding); FR-250 now defers to FR-447; QSpec #384 FR-193 and FR-193-AC-7 adopt the literal rule with the `2 - 1` example. |
| FND-006 | fixed | FR-447 -> FR-250 is now `references`; the cycle is gone. |
| FND-007 | fixed | AC-2 adds a guard `when { v + 1 }` (non-boolean-root) and a flow `v' = true;` (type-mismatch). |
| FND-008 | fixed | The clause, AC-4 and TC-1187 now say checked package identity, and the clause states the quire-canonical preimage form. |
| FND-009 | fixed | Moving's invariant now bounds v to [0, 20] and a to [1, 2], and the use case states the claim holds. |
| FND-010 | fixed | The Description is non-normative prose; make check-spec-validation shows no diagnostic on FR-447. |
