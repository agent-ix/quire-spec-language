---
id: SR-12904
title: "Failure-domain review of PR #999: shared clause nodes, lookup refusals and refused-key locus"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@546412bb9; FR-337 Outputs, Behavior identity, over and I2 reader rules, AC-5; FR-338 root paragraph, ClosureCertificate, Outputs, refused-key Behavior rule, AC-8; FR-088-AC-3; ADR-013 O-07 and Public type row; TC-1079; TC-1080; PR #997 check_closure"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---

## Summary

Ticket: STD-185. PR quire-spec-language#999. Looks for identity confusion,
refusal paths with no stated outcome, and order dependence in the new
FR-337 clause identity and lookup rules and the FR-338 refused-key rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Identity confusion: FR-337 treats one clause node as having one occurrence. The text gives "`ordinal` is the occurrence ordinal the package assigns that node", and the reader "SHALL return the clause's identity triple". Node ids are content-addressed, though, so two structurally identical clause declarations share one node and differ only by their `claim` occurrence ordinal (ADR-013 O-07 and its Public type row; FR-088-AC-3). For such a node, the reader has to return one triple per occurrence, and the clause → triple mapping is per declaration, not per node. FR-338's "rejects a foreign occurrence" depends on that distinction. AC-5 checks a single-occurrence clause only, so a reader that returns ordinal 0 for every clause passes it. Fix: say the triple is per `claim` occurrence, and add an AC-5/TC-1079 vector with two structurally identical clauses: one node, two triples with ordinals 0 and 1, each resolving, and each refused as the other's occurrence. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:59-62,97-99,115; spec/functional/FR-088-clause-name-and-type-identity.md:240; spec/functional/FR-338-check-an-en-1-closure-certificate.md:109-113 |
| FND-002 | medium | A refusal path with no stated outcome: FR-337 says the I2 reader "SHALL refuse a lookup whose node is absent, is not a temporal clause node, or whose occurrence role or ordinal differs", but names no code and no locus. Every other reader refusal in FR-337 cites FR-370's code and locus. FR-338 says the checker "rejects a foreign occurrence", but not whether a root that does not resolve is a certificate rejection (which `CertificateRule`, at which locus) or a request refusal (#997's `ClosureRefusal`). AC-5 only says "is refused", so any error at all passes it, and two implementers will settle the same unresolvable root differently: `Inconclusive(CertificateRejected)` against `Failed`. The text also gives the lookup to the I2 reader, while the rule above it gives resolution to "a consumer". Fix: name the refusal code and locus (or the FR-370 code it reuses), state FR-338's outcome for an unresolvable root, and make AC-5 assert the code. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:87-92,97-99,115; spec/functional/FR-338-check-an-en-1-closure-certificate.md:109-113 |
| FND-003 | medium | Order dependence: when a certificate holds more than one refused key, FR-338's new rule ("One refused key rejects the whole certificate") does not say which state is the `ProductState` locus. `ClosureCertificate.states` is a `Vec` with no order constraint, and #997 returns the first refused state in certificate order. Two orderings of the same certificate therefore settle to different `Inconclusive(CertificateRejected{ShapeMismatch, at})` terminal records, which goes against FR-338's order-independence stance ("Same facts reached in different materialization orders SHALL produce identical full octets"). Fix: name the reported state, for example the least refused `ProductStateRef` in ascending complete octet order, and add the two-refused-keys case to AC-8. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:46-50,544-545,562-569,599 |

## Verdict

The refused-key outcome itself is safe. The rule refuses before the key is
used for membership, expansion or acceptance, so a malformed key cannot make
a state count as covered. It rejects the whole certificate, so valid states
cannot outvote a bad one. It settles `inconclusive`/`CertificateRejected`,
which is never a proof. The restored-key half of AC-8 shows the rejection
comes from the key alone. Putting `over` as a reference with the binding
taken from the model subject keeps a package from fixing an instance, so a
new universe cannot reuse an old proof's binding.

Not mergeable as is: FND-001 to FND-003 are medium and should be fixed in
this PR. No high.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 05a741373: "One identity triple `[node, role, ordinal]` per `claim` occurrence of the clause node ... two structurally identical clause declarations share one node and differ only by occurrence ordinal (FR-088-AC-3); each declaration has its own triple". AC-5 and TC-1079 step 3 add the two-declaration vector (one node, two triples with distinct ordinals). |
| FND-002 | fixed | 05a741373: the lookup refusal is now named `clause-root-unresolved` at the looked-up triple, as an EARS If/then rule. FR-338 now states the outcome: "If the request's own clause root does not resolve ... the checker SHALL refuse the request ...; FR-127 settles it `Failed`", while a foreign context root follows the refused-key rule. AC-5 asserts the code, and AC-8 and TC-1080 step 6 cover the refused request. |
| FND-003 | fixed | 05a741373: "When more than one state's key is refused, the checker SHALL report the least refused `ProductStateRef`, ordered by the `model_state` digest octets and then the `automaton_state` octets, ascending unsigned lexicographic". AC-8 and TC-1080 step 5 cover both orderings. #997 still returns the first refused state in certificate order and has to adopt this rule. |
