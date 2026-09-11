---
id: SR-339
title: "Failure domain analysis of native population identity, anchor and closure boundaries"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md; src/protocol_artifact/native/populations.rs; src/protocol_artifact/models/populations.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Failure-domain analysis of the nominal identity, anchor and closure boundaries
introduced by the population/reference exports at `8590407`, selected alongside
the base review (SR-338) because this change is almost entirely about which
identity a requirement is attached to. Three of the four checklist areas hold as
implemented; the gaps are that the identity keys and the termination rule they
depend on are stated only as source comments and enforced invariants, not as
requirements, and that one direction of the reader's trust boundary is unguarded.

## Verdict

**CONDITIONAL** — no high finding. Object-role identity is explicit and correct:
roles are unique by `record` at admission, `universe` deliberately is not, and
the export triple and `Reaches` universe are bound by pointer identity against
one catalog rather than by label. Anchors are preserved rather than collapsed
across pre-state and capture, and both record traversals terminate on cyclic
graphs under charged bounds. The findings are unstated constraints with real
failure modes behind them, in the form this skill asks for: proposed StR/NFR
additions rather than code changes.

## Checklist

### 1. Extension points and trust boundaries

The reader is the only trust boundary added here, and its failure policy is
strict throughout — every disagreement is a typed refusal, nothing is logged and
suppressed, and no partial package is produced. The unstated half is direction:
the reader refuses a *missing* population requirement and accepts an
*unjustified extra* one (FND-001). Strict-versus-resilient is settled for the
refusal path and unspecified for the surplus path.

### 2. Entity identity

Explicit and enforced where it matters: an `ObjectRole` is unique by `record`
and by `reference` within a model (`native_model/admission.rs:461`), `universe`
is explicitly not a key, and FR-042 and the wire contract now both say so. Two
identities are left implicit — which model two roles belong to (FND-002) and
what makes two population requirements the same requirement (FND-003).

### 3. Evaluation purity

Clean, and worth recording because it is the property the whole change rests on.
Requirement derivation is a pure function of the admitted model, the typed
declaration and the layout: no observation is read, no membership is computed, no
closure truth is asserted, and the emitted `Closure` carries only a dependency on
its `Population`. The reader performs no IO and consumes no observations, which
FR-042's Inputs section already states normatively.

### 4. Topological robustness

Termination on cyclic and self-referential record graphs is guaranteed on both
sides by a visited set keyed on `(model, record, anchor)` with the field walk
performed only on first visit, so total stack pushes are linear in the model's
field count rather than in its path count. Every push and every set insertion is
charged to `Dimension::Entries` before it happens, and `Dimension::Depth` is a
peak dimension, so a deeply nested type refuses rather than recurses. None of
this is required anywhere (FND-004).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Trust-boundary direction unspecified. FR-042 says what a population requirement must be derived from and that a crossed triple must be refused; it does not say the emitted requirement set is exactly the derived set. The reader implements sufficiency only, so a package carrying an extra well-formed `Population`+`Closure` pair for any bound object role at any anchor of that declaration is admitted, and the consumer is asked for observation inputs no authored value justifies. Proposed **StR**: the admitted population/closure requirement set of a declaration equals the set derived from its non-derived binders and anchored values; a surplus requirement is refused with the same typed cause as a missing one. See SR-336 FND-001 for the code path and SR-337 FND-002 for the traceability gap | src/protocol_artifact/models/populations.rs:257; src/protocol_artifact/models/populations.rs:304; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:118 | missing-requirement |
| FND-002 | medium | Identity confusion at the model level. "The exact selected model" appears in FR-042 and the wire contract, but the uniqueness key of a model is never stated, and three different keys are in use: `model.environment().owner()` alone in the emitter's need map, owner **and** digest in `same_model`, and the wire model index in the reader. They agree only because linking refuses two selected inputs sharing an owner — different digests become `ModelConflictKind::Owner`, equal digests become `AmbiguousSelection`. That is a real invariant, enforced two modules away and cited at neither use site. Proposed **StR**: a native model's identity is its declaration owner together with its artifact digest, and no two distinct models may be selected in one declaration's bindings. See SR-336 FND-002 | src/protocol_artifact/native/populations.rs:27; src/protocol_artifact/native/populations.rs:147; src/protocol_artifact/models.rs:812; src/linking/composed/models.rs:354 | missing-requirement |
| FND-003 | low | Requirement identity unstated. What makes two population requirements the same is `(model, record, anchor)` — the reader's `pairs` key, which is also what makes a duplicate a `Invalid::Duplicate` refusal and what forces exactly one closure per population. Nothing states it. The emitted binding `name` is `population:{model index}:{export index}:{anchor}`, matching the module's existing `instance_name` idiom but carrying no role identity, so a consumer cannot recover the universe from a requirement name without resolving the export. Proposed **StR**: state the requirement key, and state that the name is an opaque per-declaration label rather than an identity | src/protocol_artifact/models/populations.rs:13; src/protocol_artifact/models/populations.rs:198; src/protocol_artifact/native/runtime.rs:643 | missing-requirement |
| FND-004 | low | Topological guarantee unstated. FR-042-AC-9 requires independently counted limits per work dimension, which covers exhaustion, but nothing requires that population derivation *terminate* on a cyclic or self-referential record graph — the difference between refusing a large model and not returning on a small one. The implementation is correct on both sides and the property is cheap to state. Proposed **NFR**: nested record/reference traversal for population derivation terminates on cyclic and self-referential record graphs, visiting each `(model, record, anchor)` at most once, and refuses through the declared work dimensions rather than growing unbounded | src/protocol_artifact/native/populations.rs:167; src/protocol_artifact/models/populations.rs:313; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:161 | missing-requirement |
| FND-005 | low | Anchor-selection rule stated only as a source comment. Two decisions carry the pre-state/capture correctness the new FR-042 paragraph claims: derived binders (`Let`/`Capture`/`Query`) mint no population, because a derived token keeps its initializer's observation; and a value with a `Selected` origin mints none, because every contributing original value is enumerated separately by the layout and would otherwise have its distinct observations collapsed. Both are sound — the layout enumerates all values, and a cross-unit `Selected` origin is refused outright — and both are load-bearing for the anchor set. Neither appears in FR-042 or the wire contract, so a future reader of the requirement cannot tell that skipping a `let` binder is required rather than incidental | src/protocol_artifact/native/populations.rs:51; src/protocol_artifact/native/populations.rs:84; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:122 | missing-requirement |
