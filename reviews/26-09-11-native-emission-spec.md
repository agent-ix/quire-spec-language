---
id: SR-335
title: "Base spec review of FR-042 after the native emission increment"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md; docs/compiled-protocol-v1.md; README.md"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Recheck at `89bc4e3` of the base checklist over FR-042 and the
`docs/compiled-protocol-v1.md` / `README.md` prose it incorporates by reference.
Base set only, no optional analysis lenses, per the owner's selection; the
optional semantic gap extension was declined. Both correctable findings are
closed: FR-042 Inputs now owns the three caller-supplied revision namespaces,
Outputs describes the public admission/emission/reader interfaces by behavior
instead of naming a refactorable Rust module path, and the wire contract's
constant-decision claim is now backed by executing tests. No acceptance
criterion, ID or trace link changed.

## Verdict

**CONDITIONAL** — no high finding. IDs, formats, links and the six coverage rules
hold. The single residual issue is criterion granularity, which the owner has
scoped as a follow-up: no renumbering campaign is approved, so AC-4/AC-5/AC-6 stay
compound for now and this review records the consequence rather than acting on it.

## Disposition of the initial findings

- FND-001 (medium, compound AC granularity): **unchanged**, carried below as
  FND-001 and scoped as an owner-approved follow-up rather than work for this PR.
- FND-002 (medium, wire-contract claim with no evidence): **resolved**.
  `docs/compiled-protocol-v1.md:502-504` still states that supported constant
  decisions establish coverage and non-overlap directly, and the claim is now
  backed: `native_owned_choice_proves_nonliteral_constant_guards_and_preserves_both_branches`
  admits a covered, non-overlapping constant decision, and
  `native_choice_refuses_overlap_uncovered_and_unproved_dynamic_decisions` refuses
  an overlapping pair and an uncovered pair. The neighbouring sentence on
  continuing bounded loops is backed by the three-case repeat test.
- FND-003 (low, Outputs naming an internal Rust module path): **resolved**.
  Outputs now reads "The public Rust admission, emission and reader interfaces
  expose structured results through the wire contract", removing
  `protocol_artifact::native::admit` / `native::emit` while keeping the versioned
  wire contract as the stable home for the interface shape.

## Checklist result

- ID format and uniqueness: FR-042, FR-042-AC-1..AC-10 and TC-121 all conform, are
  sequential and unique. No `-CON-` or `-OPT-` ids exist in FR-042, matching the
  repo-wide convention — no FR under `spec/functional/` carries Constraints or
  Options sections, a documented repo idiom rather than a gap.
- FR quality: Description, Inputs, Outputs, Behavior, Acceptance Criteria and
  Dependencies are present and specific. The added Inputs sentence is accurate
  against the public API: `SourceSelection::revision_namespace`,
  `Selections::definition_revision_namespace` and
  `Selections::requirement_revision_namespace` are exactly the three namespaces it
  names, and the separation it asserts from source-artifact revision labels is the
  behavior `semantic_definition_revision_is_independent_of_exact_source_artifact_revision`
  tests. Error conditions keep the typed `Invalid`/`Unsupported`/`Incomplete`
  vocabulary the wire contract enumerates. Related US-004 and all `depends_on`
  edges resolve.
- Six coverage rules: every AC names TC-121; error paths and boundary conditions
  are named per criterion; `quire coverage` confirms 9 of 10 ACs backed by a tagged
  test, with AC-10 correctly held open. No AC is marked complete anywhere, and
  every FR-042 and TC-121 matrix row is still `🚧 Planned`.
- Claim check: `README.md:44-48` still states that real source-to-reader tests
  exercise native predicate, state, temporal and protocol emission, and still
  lists general decision proofs, recovery admission, unavailable producer exports
  and the producer-to-B handoff as open. Both halves remain accurate at `89bc4e3`:
  the fifteen tests take real source through `admit`, `emit` and an independent
  `read`, none substitutes a manually authored wire fixture, and the open list
  matches what the code refuses through `Unsupported`.
- Cross-referencing: FR links to US, ACs link to TC-121 and `quire-protocol`
  IT-001, full IDs are used throughout, and terminology (`family admission`,
  `authority`, `unsupported prerequisite`) stays consistent across FR-042, the
  wire contract and `README.md`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Compound acceptance criteria still block expressing a partial delivery. FR-042-AC-5 bundles sequence, owned labeled choice, parallel/join, bounded progress, await branches, termination and five refusal classes into one criterion; AC-4 and AC-6 are similarly compound. Every half of AC-5 now has an executing test, so the criterion is no longer overclaiming — but the granularity problem is structural: the next partially delivered control form will again have no way to say which half is covered. Splitting AC-5 per control form remains the fix. Owner-scoped follow-up; no renumbering is approved for this PR | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:145; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:146; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:147 | wrong-requirement |
| FND-002 | low | FR-042-AC-10 is the one criterion with no tagged test, and correctly so: it requires the real `quire-protocol` public Rust admission/linking handoff, which FR-042's Dependencies section and `README.md` both hold open. Recorded so the 9/10 coverage figure is not read as drift, and so the compiler-to-reader round trip the new tests establish is not mistaken for the B-side handoff | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:151; README.md:46 | correct-requirement-no-evidence |
