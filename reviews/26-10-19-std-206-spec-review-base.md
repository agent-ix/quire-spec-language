---
id: SR-13266
title: "Spec review of PR #1038: FR-140 mapping rules, ADR-020 section 8 guard, FR-141-AC-7 (base)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@651a86903; git diff origin/main...HEAD (PR #1038)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1083
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-540
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-544
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-555
    type: reviews
---

## Summary

Ticket: STD-206. Base checklist over `git diff origin/main...HEAD` at 651a86903 (9 files). ID discipline holds: no existing AC is renumbered or reworded except FR-141-AC-7, rewritten in place; FR-140-AC-1..5 and FR-139-AC-1..5, which open code PRs #1032 and #1035 cite, are unchanged; each new AC names a Test verification and a TC whose scope was widened. No hash, digest or pin is introduced. quire validate passes on all 9 files (quire 0.36.2 / engine 0.50.2) and tools/check-index-completeness.sh passes.

Verified with the engine: the guarded ADR-020 section 8 `Inc` and `CommitAPost`/`CommitBPost` check, and each unguarded form refuses Unproved(Range{required [0,2], proved [1,3]}) at its clause, as the ADR's new paragraph says.

Not verified by the author and probed here: (1) `=` between `Int[lo,hi]` and `Integer` is refused by QSpec FR-149 and by the engine; (2) `1 div self.index` cannot be an `Int[0,1]` argument, because `div` types `Integer` with no interval; (3) no source spelling lets a unit select a division law. PR #1036 (STD-203) also adds TC-1083, and #1039 adds TC-1084, so the next free id is TC-1085.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-135 complete-value typing bullet | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 Int-Integer equality bullet | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135-AC-6 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-139 sum bound bullet | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-139 only-not-one bullet | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-139-AC-6 | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-139-AC-7 | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-140 Inputs (two systems, over_model) | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 Outputs EvalFailure and MappingRowRef prose | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 nested reference lifting bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-6 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-7 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-8 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-9 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-10 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-11 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-141-AC-7 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| ADR-020 section 8 guarded Inc and CommitAPost | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | examined |
| ADR-020 section 8 guard rationale | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | examined |
| TC-1083 | spec/test-cases/TC-1083-the-refinement-mapping-lifts-references-and-reports-what-it-cannot-map.md | examined |
| TC-540 step 6 | spec/test-cases/TC-540-s3-checks-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| TC-544 steps 6-7 | spec/test-cases/TC-544-population-valued-expressions-type-and-evaluate-inside-a-refinement.md | examined |
| TC-555 fixture | spec/test-cases/TC-555-a-refused-or-incomplete-mapping-row-leaves-the-refinement-undetermined.md | examined |
| PR #1038 body: QSpec asks | PR #1038 body | examined |
| ADR-020 section 8 RingIsQueue items row | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | context_only |
| ADR-020 RM-3 | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | New SHALL admits `=`/`!=` between Int[lo,hi] and Integer, which QSpec FR-149 (and FR-149-AC-21) refuses and the engine refuses; FR-135-AC-6 asserts the refused row checks. QSpec FR-149, 'Arithmetic operand against a bounded integer': only a binary +, - or * operand of = against an Int[..] operand is retyped; 'a path, a call, div, rem, count, sum ... and any other Integer operand against an Int[..] operand remain refused { code: ill_typed, cause: type-mismatch } without an explicit convert' (FR-149-AC-21). `(self.head + i) mod 2` is such an Integer operand (QSpec FR-147: mod yields a mathematical integer; engine types IntegerDivision as ValueType::Integer). QSL cannot add an equality rule that the QSpec typing it implements forbids. Engine probe (scratch clone of qsl-544 at 11a735627, CasRefinesCounter fixture, own CARGO_TARGET_DIR): `self.value = self.value mod 2` in a state clause refuses ill_typed/type-mismatch; a refinement field row `if self.value = self.tmpA mod 2 then 0 else 1` refuses Check(IllTyped(TypeMismatch)); `self.value = self.value mod 2 + 0` refuses Unproved(Range{required [0,2], proved unbounded}); `convert<Integer>(self.value) = self.value mod 2` checks. The `+ 0` workaround works only on the Int side (`s.index + 0 = (...) mod 2`, both Integer, which is what PR #1035's fixture uses); on the mod side it refuses unproved-range. Fix: drop the bullet and FR-135-AC-6's first half, and spell ADR-020 section 8's row `convert<Integer>(s.index) = (self.head + i) mod 2` (the explicit FR-149 equality conversion) or keep `s.index + 0`. The QSpec ask is then not 'confirm' but 'fix FR-376-AC-5', whose ring row uses the same unconvertible spelling and so contradicts FR-149-AC-21 inside QSpec. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:143-145 |
| FND-002 | high | `1 div self.index` is not a well-typed Int[0,1] argument: div types Integer with no interval, so the '[1,1] conforms' reasoning has no basis and the row refuses at S3. The AC's argument for well-typedness ('a zero divisor is undefined under every law, and the quotient of 1 by the divisor 1 lies in [1, 1], which conforms to enq's Int[0, 1] parameter') assumes a division interval rule that neither QSpec nor QSL has. QSpec FR-375 types each step argument by FR-151 conformance after reference lifting, and FR-149 says div is never retyped; the engine's IntegerDivision node has type ValueType::Integer (qsl-semantics/src/check/check/typing.rs), and Integer does not conform to Int[0,1]. Probe with a floor law supplied through PackageDeclarations.integer_division: field rows `1 div self.tmpA` and `2 div self.tmpA` against Int[0,2] both refuse Check(IllTyped(TypeMismatch)); `convert<Int[0, 2]>(1 div self.tmpA)` also refuses. Without the law they refuse MissingSelection{IntegerDivision}. The STD-206 default ('a div ... whose result range is stated') needs a typing rule first. Either take the undefined source from a form that types by its body (an `only` row, cause only-not-one, or FR-141-AC-6's `inv` history update) or ask QSpec for an interval rule for div and leave the AC blocked on it. Run the chosen spelling through the checker before landing. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:185 |
| FND-003 | high | Nested reference lifting is stated for evaluation only; FR-135, ADR-020 RM-3 and QSpec FR-375 lift only a top-level Reference<T_C>, so FR-140-AC-6's Sequence<Reference<Ring>> and Option<Reference<Ring>> rows refuse at S3. FR-135: 'a field of declared type Reference<T_A> SHALL accept an expression of type Reference<T_C> exactly when an object row maps T_A from T_C', then conforms after that lifting. RM-3 and QSpec FR-375 say the same. A row of type Sequence<Reference<Ring>>[0, 2] does not conform to Sequence<Reference<Queue>>[0, 2] under that rule, so AC-6's variant is refused ill_typed/type-mismatch before map_state runs, and the new evaluation rule is unreachable. Add the matching typing rule to FR-135 (and RM-3), with an S3 AC, and add a QSpec FR-375/FR-377 ask. The PR body lists no such ask. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:117-121 |
| FND-004 | medium | 'any integer-division law the unit selects' / 'a unit that selects one integer-division law': no QSL or QSpec text gives a source spelling, and the engine takes the law from the package, not the unit. QSL FR-091 says 'the checked package's exactly-one integer-division definition selects the div/rem law'; QSpec's definition README says 'exactly one integer-division profile only by integer_div_rem packages'. In the engine, PackageDeclarations.integer_division is set by the caller (the tests set it directly). A unit header `profile d = "quire.value.integer-division.floor/v1";` still refuses MissingSelection{IntegerDivision} (probed). FR-141-AC-7 and TC-555 cannot say how their fixture selects the floor law. Name the mechanism (the package's definition selection, FR-091), not 'the unit selects'. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:134-142 |
| FND-005 | medium | AC-6 and AC-11 rows use `Some(...)`, which is not a shared-grammar form (it has none, null, present(), value()). QSpec shared-grammar primary: 'true' / 'false' / 'null' / 'none' / 'self' / 'result' ... / ('present' / 'value' / 'deref' / 'pre'), '(', expr, ')'. There is no Some constructor, and QSpec separates absent from explicit null. Spell the option rows in the grammar (for example a T-typed expression where Option<T> is expected, and `none`), and say whether an Option field row can be `null`. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:169 |
| FND-006 | medium | AC-6's second clause, 'the same rows over a Reference<Ring> whose type no object row names', is incoherent: Ring is named by the object row; the fixture needs a named concrete subtype of Ring. The UnmappedType case arises only when a referenced object's most-specific type is a concrete type with no object row, such as a subtype of Ring in R::rings (RM-3 maps by most-specific type). The AC names no such type, no population membership and no state, so a test author must invent the variant. Name it (for example `SubRing <: Ring` in R::rings, with the object and state). | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:169 |
| FND-007 | medium | 'a row whose evaluation exceeds an evaluator limit gives EvalFailure::Refused' names no limit, row or fixture; the rest of AC-9 repeats AC-5 (budget zero -> Incomplete) and AC-3 (only -> Undefined). A meter budget gives Incomplete, so a Refused case needs a specific non-meter evaluator limit (which one? CheckingLimits, ObservationLimits, a collection bound?) and a row that hits it. As written the Refused arm cannot be tested to a definite expectation. Name the limit and row; drop or reference the parts AC-5 and AC-3 already cover, keeping only the new reason only-not-one assertion. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:172 |
| FND-008 | medium | TC-1083 assigns CasRefinesCounter to AC-7..AC-9, but AC-9's `only` exists only in RingIsQueue and AC-7's `Stray` variant is defined nowhere. CasRefinesCounter has one row, `value = self.value`, so it cannot hold an `only` that finds two objects (AC-9, which is FR-140-AC-3's RingIsQueue variant). For AC-7 the TC must say how `Stray` enters Impl::counters (a second member type of the population, or a subtype of Counter), with its key and fields. | spec/test-cases/TC-1083-the-refinement-mapping-lifts-references-and-reports-what-it-cannot-map.md:23-24 |
| FND-009 | medium | TC-1083 collides with PR #1036 (STD-203), which adds TC-1083-liveness-model-only-pass-and-unsupported-refusals.md; #1039 claims TC-1084; next free is TC-1085. Both PRs claim the same id, so the second to merge breaks id uniqueness. Renumber here (TC-1085, and the 'Tag the tests' line and FR-140-AC-6..11's Verification cells) or coordinate with #1036. | spec/test-cases/TC-1083-the-refinement-mapping-lifts-references-and-reports-what-it-cannot-map.md:2 |
| FND-010 | medium | `put(1)` names no receiver: `self.index` is the receiver Slot's index, not the slot put writes, so with receiver s1 the argument is 1 div 1 = 1 and no Undefined occurs. ADR-020 section 8: 'Slot::put(x) writes x into the slot at index (head + size) mod 2 of its ring'. That is not necessarily `self`. The claim 'the undefined case is a runtime one at index = 0, the slot that put(1) writes into an empty queue' holds only for receiver s0 and an empty queue with head = 0. Name the receiver and the empty state (head, size). | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:185 |
| FND-011 | low | The QSpec asks are real (only-not-one is absent from the closed Undefined reasons list; FR-376 says 'n times e's bounds'; shared-grammar has none of the six productions or `..`), but the `=` ask is phrased as a confirmation QSpec already answers no, and asks for nested lifting and relationship links are missing. Checked against agent-ix/quire-specification main 2c2918b: native-diagnostics.md Undefined reasons lists only absent-key and precondition-false; FR-376's table gives sum 'with bounds n times e's bounds'; shared-grammar.md has no count/sum/exists/all/only/seq binder productions. The fourth ask should be 'FR-376-AC-5 contradicts FR-149-AC-21' (FND-001). Add asks for FR-375 nested reference lifting (FND-003) and the mapped state's links (failure-domain SR). No Linear ticket carries any of the asks yet. | PR #1038 body:- |

## Verdict

Not mergeable: 3 high, 7 medium, 1 low. The ADR-020 section 8 guard is right and engine-verified, and FR-139's sum bound and only-not-one are correct (PR #1035 already builds only-not-one). But FR-135's new equality rule contradicts QSpec FR-149. FR-141-AC-7's `div` argument does not type. FR-140-AC-6 depends on lifting that S3 does not admit. TC-1083 collides with #1036.
