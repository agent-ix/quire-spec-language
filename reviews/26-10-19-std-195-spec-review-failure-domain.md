---
id: SR-12928
title: "Failure-domain review of STD-195 FR-125 instancing (PR #1013)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@9a8a05e9d; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md; context FR-104, FR-107, FR-120, QSpec FR-361"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
---
# SR-12928: Failure-domain review of STD-195 FR-125 instancing (PR #1013)

## Summary

Ticket: STD-195. PR quire-spec-language#1013 at head 9a8a05e9d. Tested the four key decisions for unstated failure modes, identity confusion and determinism: canonical order, key-level instancing over covering populations, binding type = canonical member type, and absent-key reads. Context: QSpec FR-361 and FR-331 (read-only), QSL FR-104, FR-107 and FR-120.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The key rule instances only populations that cover `T` in FR-104's sense: `T` or a supertype of `T` is a member type. A population whose member types are proper subtypes of `T` is dropped. QSpec FR-361 ("each object ... whose type conforms to `T`"), FR-125's own line 109 ("An object of a subtype of `T` conforms") and AC-7 ("a claim over a subtype-conforming population yields its objects") all include it. Failure: `over (x: Animal)` over a `Dog` population yields no instance, so the result is `NoInstance`/`inconclusive`. Worse, when another population covers `Animal`, the `Dog` keys are silently omitted and the item can settle `holds` over fewer instances than QSpec requires. Covering must be both directions: some member type is `T`, a supertype of `T`, or conforms to `T`. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:107-109,121-124,212; spec/functional/FR-104-check-state-clauses.md:263-266; quire-specification/spec/functional/temporal/FR-361-read-model-behaviours-as-temporal-traces.md:125-126 |
| FND-002 | high | The binding carries the population's canonical member type, and the rule says state objects are matched "by population and key, not by type". But FR-107 (unchanged) states "The evaluator SHALL compare references by (universe, object type, key)", and FR-120 encodes a state reference with the object's most-specific type. The PR itself notes that the binding and the state reference of one object differ in type. So `x = r`, `contains(allInstances<T>(P), x)` and `deref(y.next) = x`, for an object whose type is not the canonical member type, evaluate `false` although they name the same object. That gives wrong verdicts with no undefined record. Either have FR-107 compare a bound reference by (universe, key), or have QSL re-bind `x` at each position to the state's own reference. In either case, add an AC that compares `x` against a state reference of a non-canonical object. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:125-135; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:95-97; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:209-212 |
| FND-003 | medium | Under a covering population whose member is a proper supertype of `T` (a population of `Animal`, `over (x: Dog)`), a key whose object is a `Cat` is a member of its population, yet a read through `x` returns `absent-key` and the item settles `refuted`/`undefined-evaluation`. QSpec FR-361 says "Population membership is a defined Boolean, so a clause guards such reads with it", and that guard is `true` here, so it does not prevent the refutation. Either say how a clause guards against a non-conforming object under its key (a type test, or a membership form typed by `T`), or do not instance keys whose object can never conform. Raise it on STD-196, since the guard idiom is QSpec's. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:129-132; quire-specification/spec/functional/temporal/FR-361-read-model-behaviours-as-temporal-traces.md:130-137 |
| FND-004 | medium | The canonical member type is unique: FR-104 defines it as the least declared member type in `DeclarationKey`'s total `Ord`. It need not conform to `T`. For a population `[Animal]` and `over (x: Dog)` it is a supertype. For `[Lamp, Switch]` and `over (x: Switch)`, with `Lamp` least, it is an unrelated sibling. The bound value is then a `Reference` whose type the checked parameter `x: Reference<T>` does not admit. That value enters the obligation identity (FR-169 O-09; the replay `ContextBinding` value) and names the refuted binding (QSpec FR-361), so a counterexample names an object as a type it never had. State that subtype conformance of the bound value is waived, and why. Otherwise choose a type that conforms (`T` itself, or the least member type that conforms to `T`). | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:125-128; spec/functional/FR-104-check-state-clauses.md:242-253; qsl-replay/src/model_check/item.rs:99-113 |

## Verdict

Not mergeable: FND-001 and FND-002 give wrong instance sets and wrong equality results. Clean: canonical order is deterministic (population declaration identity text is unique per population, and universe lists refuse repeated keys, FR-120-AC-7). Instancing a key once, whatever number of member types conform, removes the double-instancing defect STD-195 names. 'Least declared member type' exists and is unique in FR-104 (`DeclarationKey` total `Ord`). The no-object case reads `absent-key`, matching FR-361's existing rule.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | A population that only declares proper supertypes of `T` is not instanced. But QSpec FR-153 admits into it members of any type that conforms to a member type ("A member type covers itself and every type that conforms to it"), so a population `[Animal]` can hold `Dog` objects, and QSpec FR-361 instances each of them for `over (x: Dog)`. The supertype-only case alone gives `NoInstance`/`inconclusive`, which is safe. The mixed case is not safe: a subject with a `[Dog]` population and an `[Animal]` population holding a `Dog` instances only the `[Dog]` keys and can settle `holds` while QSpec refutes it on the unchecked `Dog` in `[Animal]`. The stated reason ("QSL cannot tell which of its objects would conform") applies just as much to `[Lamp, Switch]` under `over (x: Lamp)`, whose `Switch` keys QSL does instance and reads as `absent-key`. Make the deviation fail-safe: when a population whose member types are proper supertypes of `T` exists, settle the item `inconclusive` with a named cause, or refuse, rather than silently omitting its keys. Alternatively, instance those keys too. Add an AC for the mixed case. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:117-124,215; quire-specification/spec/functional/type-model/FR-153-query-closed-model-environments.md:67-68,89-91 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | With supertype admission, `over (x: Dog)` instances every key of `Animals`, including keys whose object is a `Cat` at every reachable state. Any unguarded read of `x` there is `absent-key`, so the item settles `refuted`/`undefined-evaluation` whatever the property says. QSL states no guard that avoids this: it has no type-test operator, and population membership is true there. QSpec FR-361's object-based instancing would never instance such a key. This was deferred as SR-12928 FND-003, but supertype admission makes it the main path for any claim over a subtype of a mixed population, and AC-12 asserts the behaviour. Yet AC-12 and AC-8 (which asserts key instancing, `Switch` keys being instances of `over (x: Lamp)`) carry no QSL-proposed marking, while Remaining work limits the QSL-proposed rules to AC-9 to AC-11. Mark AC-8 and AC-12 QSL-proposed (STD-196: instance is a key). State in Remaining work that such a claim is refuted at a non-conforming key until QSpec rules on the guard, or settle that case `inconclusive` with a named cause rather than `refuted`. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:121-125,137-139,216,220,224-236; spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md:50-53,83-86 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 01f5d081c: 'holds T' includes populations whose member type is a subtype of T, so a `Dog` population is instanced for `over (x: Animal)`, consistent with AC-7 and line 106. The remaining supertype-only deviation is FND-005. |
| FND-002 | fixed | 01f5d081c: x reads, at each position, the state's own reference (its most-specific type, the population's UniverseId), so FR-107's (universe, object type, key) equality and allInstances membership are unaffected; AC-11 and TC-520 step 10 test `x = r` and `contains(allInstances<Thing>(P), x)` for a non-least `Switch` object. |
| FND-003 | deferred | Linear STD-196: the fix records the guard question in Remaining work and in an STD-196 comment (question 3: which guard idiom QSpec intends when a key's object exists but does not conform). The case still occurs (AC-9: `over (x: Lamp)` reading a `Switch` object is absent-key), and the guard idiom belongs to QSpec FR-361. |
| FND-004 | fixed | 01f5d081c: an instance is identified by population declaration identity text plus key, with no object type, in the obligation identity and the refuted binding; the value read conforms to T at every position where it is read. |
| FND-005 | fixed | d25057781: a population admits `T` when a member type is `T`, a subtype or a supertype of `T` (QSpec FR-153), so `Animals/k` holding a `Dog` is instanced for `over (x: Dog)`; AC-12 and TC-520 step 11 test the mixed `Dogs`/`Animals` subject. No conforming object is omitted, so no `holds` over too few instances. |
