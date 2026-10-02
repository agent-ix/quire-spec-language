---
id: SR-357
title: "Failure domain analysis of the shared recovery provenance rule"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md; src/protocol_artifact/recovery.rs; src/protocol_artifact/value_graph.rs; src/protocol_artifact/validate/control.rs; src/protocol_artifact/native/metadata.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

`spec-failure-domain-analysis` over the shared native-emission/parser-free-reader
authority introduced at `a33a3c1`. The previously recorded failure mode — two
structurally different derivations compared for exact equality, so a divergence
refuses a package this compiler just produced (SR-350 FND-001) — is eliminated
at its root rather than papered over: one traversal, one selection, both paths.
What remains are unstated constraints that today's implementation satisfies by
accident of how the emitter happens to build bindings, plus one new refusal
reachable only through emission.

## Verdict

**CONDITIONAL** — two mediums, three lows, no high. The dangerous symmetric
failure is gone; the residuals are soundness and robustness premises the
specification does not state, none of them a demonstrated current failure.

## Findings

| ID      | Severity | Summary                                                                                                        | Refs                                                                                          | Escape Cause        |
| ------- | -------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------- |
| FND-001 | low      | Entity identity: the uniqueness key of a "declaration-owned population/closure pair" is unstated in FR-042, and the crate's two agreeing implementations of it are maintained independently | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:181; src/protocol_artifact/recovery.rs:114; src/protocol_artifact/models/populations.rs:233 | missing-requirement |
| FND-002 | medium   | Topological robustness: the derivation is a graph traversal whose termination, cycle behaviour and work-exhaustion refusal are stated nowhere | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:174; src/protocol_artifact/value_graph.rs:106 | missing-requirement |
| FND-003 | medium   | The exclusion of callee-owned graphs is sound only because a callee's anchors can never own the caller declaration's bindings; that premise is unstated, and violating it under-approximates provenance permissively | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:179; src/protocol_artifact/value_graph.rs:81 | missing-requirement |
| FND-004 | low      | New emission-side failure surface: native admission can now refuse an otherwise admitted declaration with `Invalid::Binding` or `Incomplete` during provenance attachment, with no stated diagnostic expectation | src/protocol_artifact/native/metadata.rs:498; src/protocol_artifact/recovery.rs:36            | correct-requirement-no-evidence |
| FND-005 | low      | Capture-stage confusion is still caught by the type/scope stage rather than by an emitter-side detector, so an emitter regression has no independent detector (unchanged from SR-350 FND-005) | tests/native_compensation_emission.rs:181                                                      | correct-requirement-no-evidence |

### 1. Extension points and trust boundaries

The relevant boundary is emission versus reading, and the change removes rather
than adds trust: the reader no longer accepts a set derived by a rule it cannot
reproduce, because it re-derives it from the same wire data with the same code.
Failure policy at this boundary is strict throughout — every disagreement is a
refusal (`Invalid::Binding`), never a suppression or a widening. The deleted
emitter path had consulted `context.typed.nodes()` and `ObservationOrigin`,
inputs the reader does not possess; the new path reads only `values`, `binders`,
`anchors` and `bindings`, all of which are in the emitted bytes. The regression
test confirms this operationally with `read.package() == package` on both
branches.

### 2. Entity identity — FND-001

FR-042 selects "exactly the declaration-owned population/closure pairs" without
saying what makes a population and a closure *one pair*. Both implementations
nonetheless apply the same predicate. `recovery::pairs` requires
`closure.requires == [population]` plus equality of model, value type, subject,
anchor and scope, and refuses `Invalid::Duplicate` when two closures claim the
same population. The model validator reaches its members by the key
`(model, record, anchor)` and then enforces the identical linkage and the same
four field equalities at models/populations.rs:233-239, over members that have
already passed `Subject::Declaration` owner validation at
models/populations.rs:156-163. So the identity is consistent today and no
divergent-key failure mode is demonstrated — the residual is that a rule the
specification never states is kept consistent by two hand-maintained copies.
Stating the key once in FR-042, and letting one module own the predicate,
removes the class rather than the current instance.

### 3. Evaluation purity

Good, and now explicit: the specification states that source-span containment
and proof folding can neither add nor remove a dependency, and that self-selected
markers add no edge. That is the purity constraint whose absence produced the
original defect — provenance derived from a representation-dependent artefact
(span layout) instead of from the value graph. The implementation matches:
`ValueGraph::new` reads `Origin`, `ValueOperation` and binder initializers only,
and the test pins a capture initializer that lies textually outside the `recover`
region and is still followed when read.

### 4. Topological robustness — FND-002, FND-003

Termination is guaranteed in code by the `visited` set, by `validate::acyclic`
over the constructed edges (`value_graph.rs:106`), and by `Work::visit` charging
`References` per inspected node and edge, so a deep or shared graph exhausts a
declared dimension instead of hanging. None of that is stated: FR-042's new
paragraph describes what the traversal follows and nothing about what happens on
a cyclic value graph, a graph exceeding a work dimension, or an artefact whose
`recover` handle is dangling. AC-4 and AC-9 cover those generically for values
and work, which is why this is medium rather than high, but the derivation is a
new algorithm and the properties are not inherited automatically.

FND-003 is the one asymmetric risk left. Not traversing callee graphs is stated
as a rule; it is *sound* only because population and closure bindings carry
`Subject::Declaration { declaration }` pinned to the owner, so a predicate's own
anchors can never own a binding this selection could pick. If a callee ever
contributed a caller-visible observation, the rule would silently return a
smaller anchor set. That direction is permissive, not refusing: a recovery would
be attributed fewer populations than it depends on, and both producer and reader
would agree on the wrong answer — the one failure this architecture cannot
detect by cross-checking. Worth stating as a structural invariant (StR) rather
than leaving it as a property of today's binding subjects.

### 5. New failure surface — FND-004

`recovery::attach` runs inside `metadata::lower`, after a declaration is
otherwise built. Three refusals can now occur there that could not occur before:
a reachable phase anchor owned by another compensation, an unpaired or
model-less population, and exhaustion of `Entries`/`References` while building
the graph. Each returns a typed artefact cause with `work.locus` set to the
offending value, which is the right shape. But an emission-side refusal is what
a source author sees, and `Invalid::Binding` against a well-formed source names
nothing the author can act on. No test exercises any of the three from the
emission side, and no requirement says what an author should observe. If the
checking stage already forbids the source shape that triggers the first one,
that is worth recording as the reason this is defence in depth rather than a
user-facing diagnostic.
