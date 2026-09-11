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

Base checklist review of the requirement text changed by `ae910c0`: FR-042's Outputs
and Dependencies sections, plus the `docs/compiled-protocol-v1.md` and `README.md`
prose that FR-042 incorporates by reference. Base set only, no optional analysis
lenses, per the owner's selection. The edits are accurate and appropriately narrow —
they replace "planned emitter" language with the delivered path and keep every
undelivered capability explicitly open — and no acceptance criterion, ID or trace
link was changed. The residual issues are criterion granularity and one affirmative
capability claim that no test backs.

## Verdict

**CONDITIONAL** — no high finding. IDs, formats, links and the six coverage rules
hold; the criteria are correct but too coarse to express the partial delivery this
increment actually made.

## Checklist result

- ID format and uniqueness: FR-042, FR-042-AC-1..AC-10 and TC-121 all conform and are
  sequential with no duplicates. No `-CON-` or `-OPT-` ids exist in FR-042, matching
  the repo-wide convention — no FR under `spec/functional/` carries Constraints or
  Options sections, so this is a documented repo idiom rather than a gap.
- FR quality: Description, Inputs, Outputs, Behavior, Acceptance Criteria and
  Dependencies are all present and specific. Error conditions carry a typed
  vocabulary (`Invalid`, `Unsupported`, `Incomplete`) that the wire contract
  enumerates. Related US-004 and all `depends_on` edges resolve.
- Six coverage rules: every AC names TC-121; error paths and boundary conditions are
  named per criterion; `quire coverage` confirms 9 of 10 ACs backed by a tagged test,
  with the tenth correctly held open. No AC is marked complete anywhere.
- Claim check: the `README.md` statement that real source-to-reader tests exercise
  native predicate, state, temporal and protocol emission is accurate, and the
  following sentence correctly lists general decision proofs, recovery admission,
  unavailable producer exports and the producer-to-B handoff as open. The eleven
  tests do take real source through `admit`, `emit` and an independent `read`, and
  none substitutes a manually authored wire fixture for that path
  (`README.md:44`, `tests/native_protocol_emission.rs:52`).
- Cross-referencing: FR links to US, ACs link to TC-121 and `quire-protocol` IT-001,
  full IDs are used throughout, and terminology (`family admission`, `authority`,
  `unsupported prerequisite`) is consistent across FR-042, the wire contract and
  `README.md`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC granularity blocks honest partial verification. FR-042-AC-5 bundles sequence, owned labeled choice, parallel/join, bounded progress, await branches, termination and five distinct refusal classes into one criterion; AC-4 and AC-6 are similarly compound. Four native tests now carry `FR-042-AC-5`, but none of them exercises the choice or await halves that the same criterion names, and the criterion offers no way to say so. Splitting AC-5 per control form would make the delivered subset expressible | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:146; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:145; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:147 | wrong-requirement |
| FND-002 | medium | `docs/compiled-protocol-v1.md` now states that supported constant decisions establish their coverage and non-overlap directly. The code implements exactly that, but no test reaches it, so the contract asserts a delivered capability with no evidence. Either add the choice tests or move the sentence into the same "remaining implementation obligations" paragraph three lines below it. Same root cause as SR-333 FND-001 | docs/compiled-protocol-v1.md:502; docs/compiled-protocol-v1.md:506 | correct-requirement-no-evidence |
| FND-003 | low | FR-042's Outputs section now names the internal Rust path `protocol_artifact::native::admit` / `native::emit`. A requirement that cites a module path tracks a refactorable internal name; the versioned wire contract, which FR-042 already incorporates by reference, is the stable home for it. The previous text had the same shape at coarser granularity, so this narrows an existing pattern rather than introducing one | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:59 | missing-requirement |
