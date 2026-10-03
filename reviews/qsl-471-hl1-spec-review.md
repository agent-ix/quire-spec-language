---
id: SR-1277
title: "Spec review of quire-spec-language HL1: an import names its library by identity alone"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4965fe5daac7032cd45d86a39f00857b23db9b96; HL1's own commits only, git diff 2f0581fe0..4965fe5da: spec/functional/FR-099, FR-091 (AC-24), FR-098 (AC-6), FR-087 (AC-11, AC-14), spec/decisions/ADR-015, ADR-011, spec/test-cases/TC-446, TC-405, TC-444, TC-282; and the unedited text those edits must agree with (ADR-015 D-4, ADR-011 §2.1, §4 and the E4 edge row, ADR-013 conversions row, FR-087-AC-12, TC-282 body)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# Spec review of quire-spec-language HL1

## Summary

Ticket: QSL-471 (HL1), with QSL-40 and QSL-39. No PR yet (stacked on LC1).
Integrity and consistency review of HL1's spec edits, and of the unedited
text that has to agree with them. Rulings applied, not raised: identity-only
header and import, no compat path, versions are A2's.

The edits themselves are honest:

- FR-099 Behavior and AC-1/2/3/6 state the identity-only import, the
  `invalid_syntax` refusal of a trailing `version`/`digest`, D-1's new step
  order (cycle, reuse, selection, compile, view), and lock binding by the
  recomputed `package_id`. AC-3 drops the diamond clause, which the spine
  can no longer reach because `DependencyInput` admits one library per
  identity.
- ADR-015 D-1 and D-2 are rewritten to match. FR-091-AC-24, TC-405 step 4,
  TC-446 steps 2/3/6 and ADR-011's lock paragraph use the new form.

Text HL1 left behind still describes the deleted refusals, and the FR-098-AC-6
amendment accepts a weaker diagnosis that ADR-015 D-4 still rules out.

## Verdict

Request changes: one high and two medium consistency findings, plus three
low wording ones.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-098-AC-6 now expects an edited dependency source to refuse `PackageIdMismatch`, naming only the request's and the recompiled top-level `package_id`. ADR-015 D-4 still says, at rule 4, "a stale dependency's source refuses there as `DependencyIdentityMismatch` at the import that records it, naming the identity". It also says, in its closing paragraph, "a stale dependency is named by its identity". ADR-011 §5's replay bullet says the same. The spec now contradicts itself. A real diagnosis was also lost: the replay no longer says which dependency went stale, and `PackageIdMismatch`'s doc says "the source's meaning changed", which is false for the proved unit. Both halves of AC-6 can name `test/units` if rules 6 and 7 (each entry against the recompiled closure's selection) run before rule 5. Fix: either reorder the executor and D-4 so that a stale dependency refuses `DependencyIdentityMismatch` naming its identity, and restore AC-6/TC-444 to that, or amend D-4 rule 4, its closing paragraph and ADR-011 §5 to the `PackageIdMismatch` outcome. The reorder keeps a behaviour D-4 asks for. | spec/functional/FR-098-execute-a-replay-request.md:151; spec/decisions/ADR-015-compile-and-replay-against-dependencies.md:198-220; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:305-312; qsl-replay/src/execute.rs:692-728 |
| FND-002 | medium | FR-087-AC-12 still requires `ConflictingDefinition` to classify to I2's second rule, and `StaleDependency{ByteDigestMismatch}` to I2's first rule, as variants `resolve_libraries` can return. HL1 deleted `LibraryRefusal::ConflictingDefinition`, and `resolve_libraries` can no longer return `StaleDependency`. TC-282 keeps both in its body (steps 1, 3 and the classification table) and appends an "Amendment" section that contradicts them. Fix: edit FR-087-AC-12 and TC-282's steps and table to the current variant set, and drop the appended Amendment. | spec/functional/FR-087-typestate-and-cross-package-node-key.md:603; spec/test-cases/TC-282-resolve-libraries-refusals-map-to-i2-rules.md:27,41,60-61,89-99 |
| FND-003 | medium | ADR-011 still says E4 refuses `DependencyIdentityMismatch`. It says so in the lock paragraph ("a dependency whose recomputed `package_id` differs from its import's (`DependencyIdentityMismatch`, §4)"), in the E4 row of the edge table, and in §4's dependency binding ("E4 and the layer-6 `replay` facade at E9 check this ... otherwise refuse with `DependencyIdentityMismatch`"). HL1 deleted that E4 check and `LinkRefusal::DependencyIdentityMismatch`. HL1 edited the sentence right before the lock-paragraph clause but left the clause. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:416,533-537,618-626 |
| FND-004 | low | FR-087-AC-11 still says "`LibraryRefusal`'s own variant set is likewise unchanged". HL1 removed `ConflictingDefinition` from it. | spec/functional/FR-087-typestate-and-cross-package-node-key.md:602 |
| FND-005 | low | ADR-013's conversions row still says "An `import`'s digest is read as a `quire.package.semantic/v2` `DigestRecord`, a claim compared with the recomputed `package_id` ... (ADR-015 D-2)". D-2 now says an import has no digest. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:164 |
| FND-006 | low | FR-087-AC-14 still says "Binding an `Import`'s identity and digest to the source's resolved import declarations is FR-099's". `Import` has no digest now. | spec/functional/FR-087-typestate-and-cross-package-node-key.md:605 |
