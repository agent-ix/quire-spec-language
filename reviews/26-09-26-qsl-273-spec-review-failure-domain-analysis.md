---
id: SR-664
title: "QSL-273 failure-domain review of state clauses on the spine"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@d8b74aba7d349ccb3989583cc4e608aad301c38b; spec/functional/FR-104, FR-106, FR-107, FR-109; spec/decisions/ADR-012-semantic-family-extension-contracts.md (§15.1, §15.5, §15.7); spec/decisions/ADR-014 (§4, unchanged); spec/test-cases/TC-466; quire-specification@0d53cf2 proposals/quire-v1/state-contract.md; code qsl-semantics/src/check/facts.rs, qsl-eval/src/value/expression/s6a.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: reviews
---

## Summary

Ticket: QSL-273 (PR agent-ix/quire-spec-language#462). This review looks for
unstated failure modes, identity confusion across observations, and places
where the spec departs from the QSpec state contract (`state-contract.md`,
"Operation anchors, aliases and captures" and "Finite graph extension").

Clean: `reaches` follows the contract. It needs one or more steps, tests
before it suppresses a repeat, and expands each identity once, so an isolated
`a` does not reach itself. Meter exhaustion is `Incomplete`, never `false`.
The `pre(result)` and `pre(parameter)` refusals match the contract table.
Admission reads no ambient state.

Verdict: changes requested (five medium).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Edge and value shapes go past the QSpec extension. The state contract admits a `reaches` edge only of type `Ref(T)`, `Option(Ref(T))` or `Seq(Ref(T),N)`, and says "Sets, bags, OrderedSet ... remain outside this selected extension". FR-104 admits "a collection of `Reference<M::T>`". FR-107 follows edges "in declared order", which a set or bag does not have. FR-106's snapshot values take set, bag and ordered-set kinds. Failure scenario: `reaches` over a `Set<Ref(T)>` edge checks. Two evaluators expand the set in different orders, charge differently, and hit meter exhaustion at different budgets, so one input gives `Incomplete` in one and `Completed` in the other. Fix: restrict the edge to sequence and option, and refuse set, bag and ordered-set values at admission, or ask QSpec for the extension. | spec/functional/FR-104-check-state-clauses.md:97-101; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:77-83; spec/functional/FR-106-admit-snapshots-and-invocations.md:96-101 |
| FND-002 | medium | The invariant anchor departs from the QSpec state contract. The contract sets an invariant's `self` to the "exact named initialization/handler observation's current snapshot" and says "a bare context type or arbitrary current snapshot is not a substitute". FR-106's `Current { snapshot, self }` accepts any current snapshot, and ADR-012 §15.1 treats that as in scope. Native-run/1 does the same, so this is parity. The departure is still not recorded. Failure scenario: a CG or RT consumer built to the QSpec contract refuses the same invariant selection that QSL admits, and the two disagree on the ConfigVersion corpus. Fix: record it as a QSpec delta on the QSpec ticket, or name the binding that makes a snapshot the invariant's observation. | spec/functional/FR-106-admit-snapshots-and-invocations.md:125-128, 148-153; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1113-1133 |
| FND-003 | medium | Guard facts can cross observations. The contract refuses `present(pre(self.parent)) implies value(self.parent)` ("cross-observation presence fact"), and refuses `let v = self.version in pre(v)` and `let s = self in pre(s.version)`. FR-104 states none of these and has no AC for them. It relies on the "definedness cause it has today", and today no presence fact is derived for these paths at all (SR-660 FND-001). The obvious fix for SR-660 FND-001 is to add `Attribute` stable paths. Keyed without the observation, that admits the cross-observation guard. Failure scenario: `post P ... { present(pre(self.parent)) implies deref(value(self.parent)).versionNumber > 0 }` checks. On an invocation whose post sets `child.parent` absent (inside the frame for an operation that modifies `parent`), S6a reaches `value` on an absent post field. The result is a truth-less `Undefined`, where the contract requires a check-time refusal. Fix: key presence and interval facts by observation, apply the QSL-228 captured-alias rule to model reads, and add the three contract rows as FR-104 adverse ACs. | spec/functional/FR-104-check-state-clauses.md:91-96, 104-107 |
| FND-004 | medium | The extent is `Bounded` for clauses that range over an unbounded object universe. FR-104 and ADR-012 §15.7 compute the extent over the types of `self`, `result` and the parameters, and conclude that "a clause over bounded fields is `Bounded`". An invariant holds for every object of `T`, and `reaches` walks as far as the population allows. ADR-014 §4 lists "population with `Population(None)`" as an unbounded domain, and `config_history` declares no maximum. Failure scenario: once IR lowers `state` nodes, a bounded-only backend is told `NoCycle` is `Bounded`, proves it over a small fixed heap, and settles it `supported` with no bound requested. Fix: treat the clause's context population (and the traversal's population) as a `Cardinality`-boundable domain, or record it as an ADR-014 open question and mark the extent provisional. | spec/functional/FR-104-check-state-clauses.md:117-120; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1230-1239 |
| FND-005 | medium | `reaches` in a function body has no evaluator. FR-104 admits `reaches` "in a state clause or a function body", and TC-466 step 3 calls `function r ... { reaches(x, y, parent) }` through `call`. FR-102 gives `Reaches` to `StateModel`, FR-107 adds only a `ProtocolClause` S6a arm (the one existing arm is `Value`, qsl-eval/src/value/expression/s6a.rs:93-96), and FR-107 defines `reaches` only under "the observation in force" of a clause. Failure scenario: TC-466 step 3's `call` reaches a `Reaches` node that the `Value` evaluator has no arm for. The result is `InternalFault`, or an ad hoc traversal whose charging differs from the clause path, so the step 3 meter sweep gives two thresholds for one query. Fix: say which evaluator runs `Reaches` in a function body over a single `ObjectEnvironment`, with the same order and charging, or refuse `reaches` outside a state clause. | spec/functional/FR-104-check-state-clauses.md:97; spec/functional/FR-102-build-state-clause-forms.md:79-81; spec/test-cases/TC-466-s6a-evaluates-state-clauses.md:33-40 |
| FND-006 | low | Integer spellings are not canonical. Values are decimal strings, but FR-106 does not say whether `"01"`, `"+1"` or `"-0"` are admitted. Failure scenario: two snapshots with equal content and different spellings admit to equal values under different `sha256-jcs` digests, so a replay pinned to one digest refuses the other. Fix: use FR-038's integer spelling and refuse any other with `invalid-value`. | spec/functional/FR-106-admit-snapshots-and-invocations.md:68-69, 96-98 |
| FND-007 | low | A label mismatch is misreported as a digest mismatch. FR-106 check 1 sends "four non-blank labels equal to the selection's" failures to `stale_dependency`/`byte-digest-mismatch`, even when the bytes match their digest. Failure scenario: a caller whose document is intact but whose selection names another revision is told the bytes are corrupt. `revision-mismatch` is the catalog cause for a label disagreement. | spec/functional/FR-106-admit-snapshots-and-invocations.md:137-146 |

## Verdict

Changes requested. FND-003 and FND-005 cause truth-less or faulting results
at run time. FND-001 and FND-002 are undeclared departures from the QSpec
contract.
