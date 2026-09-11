---
id: SR-338
title: "Base requirements review of native population and reference exports"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Base checklist review of the specification change in `agent-a/native-population-exports`
at `8590407`: one normative paragraph added to FR-042's Behavior section, one
control block added to TC-121 step 4, and three paragraphs added to the
`compiled-protocol-v1` wire contract. The selected review set is base plus
failure-domain (SR-339); the optional semantic gap extension was declined. No
applicable `AssuranceProfile` with a `review_selection` is installed in this spec
scope, so no profile-required set applies and this direct path is the correct one.

## Verdict

**CONDITIONAL** — no high finding. IDs, cross-references and the six coverage
rules hold: no new or duplicated ids, every FR-042 acceptance criterion still
names TC-121, and the added TC-121 controls are measurable, one-axis and paired
with an expected typed refusal. The findings are traceability granularity, not
contradiction: the new normative text is not bound to an acceptance-criterion id,
its refusals are stated without their typed causes, and the matrix narrative that
bounds TC-121's evidence is stale.

## Checklist results

- **ID format and uniqueness.** `FR-042`, `TC-121` and `FR-042-AC-1..AC-10`
  conform; no id was added, renumbered or duplicated. Correct: this change needed
  no new id.
- **Functional requirement quality.** Inputs and Outputs already cover the new
  path — Inputs names the exact model/definition/dependency selections and states
  that the reader "consumes no concrete workflow instances, observations, clocks,
  assessment requests or backend results", which is exactly the boundary the
  population work must not cross, and the new paragraph restates it as "future
  membership and completeness remain consumer inputs rather than compilation
  prerequisites". Behaviour is specific: derivation source (the admitted object
  role, selected model, universe, original evaluation anchor), the refusal rule
  for a crossed triple, and the pre-state/capture retention rule.
- **Test coverage, six rules.** Coverage: every AC still has TC-121. Error path:
  the added TC-121 text requires one-axis substitution of reference, object,
  population export, model owner and source locus, each with "the corresponding
  typed refusal" — the implementation supplies a superset (eight axes, adding
  declaration owner and closure pairing). Constraint boundary: no new numeric
  constraint is introduced; the population traversal reuses the existing
  `Entries`/`Depth` dimensions already covered by FR-042-AC-9. Edge case: two
  roles sharing a universe label is named explicitly in both FR-042 and TC-121.
- **Cross-referencing.** FR-042 → `docs/compiled-protocol-v1.md` and TC-121 →
  FR-042 links are intact and the added wire-contract text matches the added
  requirement text; terminology is consistent ("object role", "universe",
  "population export", "closure requirement") across all three documents.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The added paragraph states four new SHALLs — derive the population export and runtime input requirements from the exact admitted role; refuse a crossed reference/object/population triple; retain pre-state and capture origins; keep membership and completeness as consumer inputs — but the acceptance-criteria table was not extended and no criterion names them. The behaviour is covered in substance by FR-042-AC-4 (nominal identities, graph-export authority, pre/post/capture origins) and FR-042-AC-7 (typed reader refusals), and the five new tests tag exactly those, so nothing is untested; the mapping is implicit and a reader of the AC table alone cannot see that the crossed-triple refusal is a gated criterion | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:118; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:156; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:159 | missing-requirement |
| FND-002 | low | The new refusals are specified without their typed causes, continuing FR-042's inherited absence of an error-conditions section — the document has Description, Inputs, Outputs, Behavior, Acceptance Criteria and Dependencies only. The checklist item "error conditions documented (codes)" is therefore unmet for this change: the actual causes (`Invalid::Type` for a crossed triple or a wrong edge field, `Invalid::Binding` for a broken closure pair, `Invalid::Owner` for a substituted declaration subject, `Invalid::ForeignLocus` and `Invalid::Selection` for a tampered role locus) exist only in the `Invalid` enum, the wire contract's refusal list and the tests. The wire contract's own `Invalid` enumeration at `docs/compiled-protocol-v1.md:460` is the catalog to point at; the added paragraph names no cause at all | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:118; docs/compiled-protocol-v1.md:460; src/protocol_artifact/mod.rs:148 | missing-requirement |
| FND-003 | low | The matrix prose bounding TC-121's evidence is stale and is the one place a reviewer would look to size the claim. Recorded in full as SR-337 FND-003 and cross-referenced here so the base verdict accounts for it: the section names "Twenty public reader/encoder controls in `tests/protocol_artifact.rs`" where that file now holds 24, and does not mention the two native emission suites that supply the other 20 TC-121-tagged tests. The `🚧 Planned` statuses below it remain correct | spec/model-linking/tests.md:192; spec/model-linking/tests.md:194 | wrong-requirement |
