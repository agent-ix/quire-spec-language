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
