---
id: SR-12903
title: "Integrity review of PR #999: FR-337 clause identity against FR-338, FR-125, FR-088, ADR-013 O-07 and PR #997"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@546412bb9; FR-337 Outputs, Behavior, AC-1, AC-5, Dependencies; FR-338 Inputs, canonical translation root and binding paragraph, admission negatives, Outputs, refused-key Behavior rule, AC-5, AC-8; FR-125 instance rules and AC-7; FR-127 V-6; FR-088-AC-3; ADR-013 O-07 and Public type row; TC-1079; TC-1080; PR #997 qsl-replay certificate/closure.rs and model_check/item.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---

## Summary

Ticket: STD-185. PR quire-spec-language#999. Checks that FR-337's new
identity triple, `over` node and I2 reader rules agree with FR-338's
automaton-state context, FR-125's instances, ADR-013 O-07 and FR-088, and
that FR-338's new refused-key rule agrees with FR-338 Outputs, FR-127 and
what PR #997 (QSL-529) implements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-337's new `over` rules assume every clause has an `over` parameter: the Outputs give "the NodeId of the parameter node the first argument ... references", and the reader "SHALL return the clause's identity triple and `over` node". FR-125 gives a clause with no `over` parameter one instance with no bound object (FR-125-AC-7: `possible Anything using v { true }`). FR-338 gives such an item `binding: []`, and #997's `ClauseIdentity.over` is `Option<WireNodeId>`. FR-337 does not say what the first argument holds for such a clause, or what the reader returns for it (no `over` node, not a refusal). An implementer who follows the text literally refuses the lookup for every parameter-free clause, because its first argument references no parameter node. Fix: state the no-`over` case in Outputs and the reader rule, and add a vector for it to AC-5. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:63-64,93-99,115; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:127-128,186; spec/functional/FR-338-check-an-en-1-closure-certificate.md:115-119 |
| FND-002 | low | The rule "The emitter SHALL emit the clause node as the only source of the clause's identity ... No consumer supplies either from outside the package" contradicts its own first clause. The consumer does supply the root `[node, role, ordinal]` it resolves: it comes from the request or the certificate's key context (FR-338). What the rule means is that a consumer resolves the root against the package and takes the `over` node only from the package. That is also the change #997 has to make, since its `CertificateRequest` carries a caller-supplied `ClauseIdentity.over`. Reword the rule to say that, for example "No consumer takes the `over` node, or a clause the package does not hold, from outside the package." | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:87-92 |
| FND-003 | low | FR-337's Dependencies section is not updated. The new text depends on FR-125 (the `over` binding comes from the model subject), FR-338 (the triple is its clause root), ADR-013 O-07 (the ordinal) and QSpec FR-322's NodeId, but Dependencies still lists only ADR-011, ADR-018, FR-123, FR-093 and FR-092. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:59-66,93-96,117-122 |
| FND-004 | low | The new FR-338 rule says the checker rejects a key "that the translation refuses to admit". FR-338's context includes the item's concrete `over` binding, so an item with several instances has one translation context per instance (#997 spec gap 2). "The translation" is singular, so the rule does not say which translation must admit a key: any instance's (what #997 does: `instances.iter().any(... admit ...)`), or the one instance the state belongs to. Say "that no instance's translation admits". | spec/functional/FR-338-check-an-en-1-closure-certificate.md:115-117,562-569 |

## Verdict

Consistent where the ticket asked to check:

- **Identity triple.** FR-337's `[node, role, ordinal]` with `node` the
  FR-322 NodeId and role `claim` matches FR-338's root (`node` is "FR-322's
  existing NodeId object", role and ordinal the O-07 occurrence role and
  ordinal) and ADR-013's "one `claim` occurrence per declaration". FR-338's
  new cross-reference to FR-337's clause node closes the loop.
- **`over` as a reference.** "a reference to the parameter node, never as a
  value; the concrete binding ... comes from the model subject (FR-125)"
  agrees with FR-337-AC-1 ("over (a reference to `c`'s parameter node)"),
  FR-125-AC-7 (one checked clause yields per-universe instances without
  re-checking) and FR-338's `binding` taken from the actual concrete binding.
- **FR-338-AC-8 outcome.** `Rejected{ShapeMismatch}` at
  `CertificateLocus::ProductState` with the refused key and its model state
  is what #997's `check_closure` does: it admits each certificate state's key
  before any membership use, and on failure returns
  `rejected(CertificateRule::ShapeMismatch, state.clone())`, a
  `ProductStateRef{model_state, automaton_state}`. One refused key ends the
  check, so "one refused key rejects the whole certificate" also matches.
  #997's `settle_model_check` maps `Rejected` to
  `Inconclusive(CertificateRejected{rule, at})`, which is FR-127 V-6. The
  outcome is never `Proved`, so it keeps the planner's default ("never a
  proof"). `TemporalCertificateCheck` has no `Unproved` variant, and
  `inconclusive`/`CertificateRejected` is FR-127's existing closest outcome,
  so the author's choice is sound and needs no new vocabulary. The refusal is
  of the certificate (a malformed key the engine sent), not of the request,
  so `ShapeMismatch` is better than a `ClosureRefusal`.

Follow-ups for #997 once this lands (implementation, not this PR):
`ClauseIdentity` should be resolved from the recompiled package's
`temporal_clause` node rather than supplied, and AC-8 needs a
checker-level test for the non-canonical and unsorted negatives (#997
exercises only the foreign context and foreign digest domain at checker
level).

Not mergeable as is: FND-001 (medium) should be fixed in this PR. No high.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-337-AC-5's new no-`over` vector names "FR-125-AC-7's `possible Anything using v { true }` shape". That is a state-graph claim (ADR-022 SG-4: "a property family separate from temporal claims"), and it emits no `temporal`/`temporal_clause` node, so TC-1079 step 3 cannot produce the clause it is meant to read. A temporal claim without `over` does exist in the corpus, for example FR-153-AC-3's `eventually holds(forall x in toggles: x.fired)`, which "has no `over` parameter". However, QSpec's shared-grammar `temporal-clause` requires `'over', '(', parameter, ')'`, and QSpec FR-370 always writes argument 1 as "a `reference` to the `value`/`parameter` node of the clause's `over` parameter", with a missing argument failing the schema (FR-370-AC-1). So no conforming package can hold the parameter-free clause this vector reads. Fix: point the vector at a temporal shape such as FR-153-AC-3's, and mark that half of AC-5 (and TC-1079 step 3) as waiting on the QSpec form, with a `Remaining work:` reference to the STD follow-up. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:132; spec/test-cases/TC-1079-a-temporal-clause-node-carries-its-root-and-over-node.md:23-25,35-37; spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:93; spec/functional/FR-153-decide-each-fairness-on-the-annotated-quotient.md:99 |
| FND-006 | low | FR-337 Outputs still opens with "One v2 node per temporal clause", while the new bullet right after it says two structurally identical clause declarations "share one node". Make the first bullet "One v2 node per distinct checked clause", to match ADR-013's "one node per distinct clause, one `claim` occurrence per declaration". | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:58-67 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The QSL side is fixed in 05a741373: Outputs ("A clause with no `over` parameter ... has no `over` node; that absence is a valid result, not a refusal") and the reader rule ("or no `over` node for a clause without an `over` parameter"). What the emitter writes in argument 1 for such a clause is still undefined, and QSL cannot define it alone. QSpec FR-370 and the shared-grammar `temporal-clause` require `over`, so the slot needs a QSpec (STD) follow-up, which the coordinator is filing. FR-337 should cite it as `Remaining work:` (see FND-005). |
| FND-002 | fixed | 05a741373 replaces the self-contradicting "only source" rule with an emitter rule ("the identity triple and `over` node are derivable from the package alone") and a separate consumer rule ("it takes neither a clause the package does not hold nor an `over` node from its caller"). |
| FND-003 | fixed | 05a741373 adds a `depends_on` FR-125 edge, and Dependencies now lists FR-125, FR-338, ADR-013 O-07 and QSpec FR-322. |
| FND-004 | fixed | 05a741373: "If an `AutomatonStateKey` in a certificate state is admitted by no instance's translation context (... an item's instances each have their own context)". This matches #997's `instances.iter().any(... admit ...)`. |
