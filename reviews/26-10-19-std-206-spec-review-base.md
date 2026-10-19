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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | high | The new S3 refusal of any row whose type nests a Reference also refuses a population-reference row (Set<Reference<T_C>>), which FR-139 admits and PR #1035 builds (`cells = R::slots`). FR-139 S3: 'SHALL record each bound that does (a count or sum result, or a population reference, against the type of its use)', and QSpec FR-376 types `C::p` as `Set<Reference<T>>[0, n]` in row position. PR #1035 (QSL-546) has the row `cells = R::slots` mapped to `Q::Queue.cells` with `population Q::cells from R::slots`. Under the new bullet that row refuses unsupported_construct/not-yet-implemented. Either exempt the population reference, where each element is lifted by key (the FR-139 case), or say FR-139's population-reference row is refused too and remove it from FR-139 and #1035. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:137-142 |
| FND-013 | high | The fix swaps the untypeable step argument for FR-138's `inv` update `1 / x`, which is the same `/` against an integer target that FR-141-AC-7 now says does not type. FR-138-AC-5 declares `inv: Int[0, 1] = 0 { on Impl::Register::write: 1 / x; }`. FR-141-AC-7 says '/ is refused against an integer target'; the review-pass probe agrees (a row `2 / self.tmpA` against Int[0,2] refuses Check(IllTyped(TypeMismatch))). FR-144-AC-6 (now FR-142-AC-8's `inv` update), TC-555 step 3 and FR-141-AC-7's 'FR-141-AC-6's undefined history update covers the engine-written Undefined' all now rest on it. History rows are outside the build's support set, so I could not probe the update directly. Use a source that types, such as the pure-function `mod` form in FND-014, or record the dependency on QSL-545 and STD-211. | spec/functional/FR-144-request-and-settle-a-refinement-item.md:133 |
| FND-014 | medium | A typeable undefined argument exists, contrary to FR-141-AC-7's 'pending' text and STD-211 item 8 ('no conforming source exists'): a pure function with a `mod` by its Int argument. Engine probe (scratch clone of qsl-544 at 11a735627, CasRefinesCounter): with `function f using v(x: Int[0, 2]): Int[0, 2] pure { if 1 mod x = 0 then 0 else 1 }` the row `value = f(self.tmpA)` checks with no division law selected, and map_state returns MappingFailure::Undefined naming the `value` row and object c, cause Kernel(DivisionByZero), at tmpA = 0, and maps at tmpA = 1. For FR-141-AC-7: `function g using v(i: Int[0, 1]): Int[0, 1] pure { if 1 mod i = 0 then 0 else 1 }` and `put -> enq(self.ring, g(self.index))` give division-by-zero at index 0, with no division law (mod is always Euclidean, QSpec FR-147). Restore the vector with that spelling, naming the receiver slot s0 and the empty state, and correct STD-211 item 8. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:187 |
| FND-015 | medium | FR-140-AC-11's Verification cell 'Pending (TC-1086 step 7)' is the only such cell among about 2,400 FR ACs; the AC states it has no vector, though its None half is writable in the grammar. Every other FR AC is Test, Inspection or Analysis. An AC whose own text says 'No vector exists' cannot fail, and the matrix lists TC-1086 step 7 as covering it. The shared grammar has `none` as a primary, so a `head = none` row is writable. Only the engine's S2 refuses it today (probe: S2 UnrepresentedConstruct{Primary} at `none`). Either make AC-11 a Test of the None half for both `T?` and `Option<T>`, or drop the AC and record the rule under 'Remaining work:' with STD-211. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:205 |
| FND-016 | medium | The two new unsupported_construct/not-yet-implemented refusals bypass FR-135's own support-set mechanism: no feature name, not in the closed set, and not QSpec FR-375 features. FR-135 'Refinement features the build does not check': 'QSpec FR-375 owns the closed set of feature names ... It names three features', and the refusal 'names the feature, the refinement's name and the exact selected profile identity (QSpec FR-272's payload)'. The new refusals name no feature and are not in the set. FR-341-AC-5 classifies this pair `unsupported` (a QSL implementation gap), but a relationship refusal is a gap in QSpec, which defines no link mapping. Add feature names to the support set (and STD-211), or use a refusal that does not claim an unbuilt QSpec feature. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:143-148 |

## Dispositions

Round 1, head 72d04b1dc (fix commits d561b72e7, 72d04b1dc).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
| FND-009 | fixed | - |
| FND-010 | fixed | - |
| FND-011 | fixed | - |

Disposition pass 1 (head 72d04b1dc): FND-001 to FND-011 fixed. New: FND-012 (high), FND-013 (high), FND-014 (medium), FND-015 (medium), FND-016 (medium). Not mergeable.
