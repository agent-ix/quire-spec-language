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

Failure-domain recheck at source commit `84aec59`, selected alongside the base
review (SR-338) because this change is almost entirely about which identity a
requirement is attached to. Four of the five initial findings are closed: the
unguarded direction of the reader's trust boundary is now guarded in code and
stated in both FR-042 and the wire contract, and the requirement key, the name
opacity, the termination property and the anchor-selection rule are now written
down rather than living as source comments. The fifth is corrected — its proposed
StR overreached — and retained as a low residue. No StR or NFR artifact was
created: proposed profiles are not authorized prototype gates, and the owner's
directive is against a new artifact campaign, so the constraints landed as
normative FR-042 and interchange-contract text instead.

## Verdict

**CONDITIONAL** — no high finding, and the two mediums that carried the original
verdict are gone. Object-role identity remains explicit and correct, anchors are
preserved rather than collapsed across pre-state and capture, both record
traversals terminate on cyclic graphs under charged bounds, and the reader now
refuses in both directions. The residue is a single uncited use site of an
invariant enforced two modules away.

## Disposition of the initial findings

- **FND-001 (medium, trust-boundary direction) — resolved.** The reader now
  checks the reverse direction of its existing normalized traversal
  (`models/populations.rs:365-379`), so a valid-but-surplus `Population`+`Closure`
  pair refuses `Invalid::Binding` at the surplus population's binding locus.
  Strict-versus-resilient is now settled in the same direction on both halves of
  the boundary, and FR-042 plus the wire contract state it normatively. Code path
  and red-test evidence: SR-336.
- **FND-003 (low, requirement identity unstated) — resolved.** The wire contract
  now states that within each declaration the population requirement key is the
  exact selected model, object record and original observation anchor, and that
  binding names are opaque labels (`docs/compiled-protocol-v1.md:112-114`). That
  is exactly the reader's `pairs` key, the thing that makes a repeat an
  `Invalid::Duplicate`, and the reason a consumer must resolve the export rather
  than parse a name.
- **FND-004 (low, topological guarantee unstated) — resolved.** "Nested
  record/reference traversal visits each key once, including cycles, under the
  declared work limits" (`docs/compiled-protocol-v1.md:118-119`) states the
  termination property that FR-042-AC-9's exhaustion limits do not imply. Stated
  in the interchange contract rather than as a new NFR, deliberately.
- **FND-005 (low, anchor-selection rule as source comment) — resolved.**
  "Derived binders retain their initializer's observation; selected values retain
  their contributing origins without adding a population at the selection site"
  (`FR-042:127-129`) makes the two load-bearing decisions requirements rather
  than incidental implementation choices.
- **FND-002 (medium, model identity) — corrected and retained below.**

## Checklist

### 1. Extension points and trust boundaries

The reader remains the only trust boundary added here, and its failure policy is
strict throughout — every disagreement is a typed refusal, nothing is logged and
suppressed, no partial package is produced. Both directions are now covered:
missing and surplus pairs refuse with the same cause, separated by locus. The
locus attribution itself is unasserted by any test (SR-336 FND-005), which is an
evidence gap rather than a boundary gap.

### 2. Entity identity

An `ObjectRole` is unique by `record` and by `reference` within a model
(`native_model/admission.rs:461`), `universe` is explicitly not a key, and both
FR-042 and the wire contract say so. Requirement identity is now stated
(FND-003, closed). Model identity remains the one implicit key (FND-002 below),
now documented at the emitter's use site but not at the solver's. Worth
recording from this recheck: the emitter keys on declaration owner while the
reader keys on wire model index, and those agree because distinct owners map to
distinct indices — the guard is against two *same-owner* selections, not against
two distinct model owners in one declaration, which are supported.

### 3. Evaluation purity

Unchanged and clean. Requirement derivation is still a pure function of the
admitted model, the typed declaration and the layout: no observation is read, no
membership computed, no closure truth asserted. The new reverse check adds no
input — it consults the same `seen` set the forward walk already built.

### 4. Topological robustness

Unchanged and now required. Both sides key a visited set on
`(model, record, anchor)` and perform the field walk only on first visit, so
total stack pushes are linear in the model's field count rather than its path
count; every push and set insertion is charged to `Dimension::Entries` before it
happens, and `Dimension::Depth` is a peak dimension. The reverse check reuses
that same `seen` set and charges one visit plus the key's bytes per offered
pair, so it adds a linear charged pass and no new unbounded accumulator.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | low | Corrected and narrowed. The proposed StR recorded initially — "no two distinct models may be selected in one declaration's bindings" — was wrong and is withdrawn: distinct model owners in one declaration are supported and are what the emitter's owner-keyed map relies on. The actual guard is narrower and intentional: `ModelBindings` refuses same-owner/different-digest inputs as `ModelConflictKind::Owner` and duplicate exact selections as `AmbiguousSelection`, so within one declaration a selected owner identifies exactly one admitted model and the emitter's owner key and the reader's model index stay bijective. The emitter now cites this at its `Key` type. What remains unstated is the model uniqueness key itself — `same_model` uses owner **and** digest while the emitter uses owner alone — and the second dependent site, `checking/composed/solver/validation.rs:265`, which admits a graph edge whose leaf reference lies in another model with an equal owner and cites nothing. If the linking guard ever relaxes, that is the site that widens silently | src/protocol_artifact/native/populations.rs:27; src/protocol_artifact/native/populations.rs:150; src/protocol_artifact/models.rs:812; src/linking/composed/models.rs:354; src/checking/composed/solver/validation.rs:265 | missing-requirement |
