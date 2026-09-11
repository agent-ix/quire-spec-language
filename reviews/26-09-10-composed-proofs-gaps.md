---
id: SR-328
title: "Composed guarded-definedness delivery gaps against FR-040 and TC-119"
type: SpecReview
analysis: gap-analysis
scope: "FR-040; TC-119; TM-003; src/checking/composed/proofs/; src/checking/proof/; tests/composed_proofs.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/TM-003, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-119, type: references }
---

## Summary

Claude Opus performed the targeted QUOIN gap recheck after PR #51's corrections.
The coordinating agent transcribed its findings and executed the requested local
gates. The public proof stage now has sixteen tests, including refused
completeness and actual upstream numeric refusal ownership. The earlier
missing-test high for unreachable ValueRepresentation paths is withdrawn.

## Verdict

**FAIL for full FR-040/TC-119 acceptance.** Ordered-query proof/execution,
complete family/runtime admission and the real source-to-B producer path remain
unimplemented. These are retained implementation/acceptance gaps, not grounds
for redefining the requirement or stopping other prototype engineering.

**Delivered proof stage: PASS.** No outstanding gap was found in the supported
stage claimed by this PR. Code review retains two low notes in
[SR-327](26-09-10-composed-proofs-code.md). Tagged partial controls do not
establish complete acceptance of their owning criteria.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Full TC-119 query, family-context and runtime groups remain incomplete; producer-to-B emission is not delivered by this proof stage. | spec/test-cases/TC-119-check-composed-values.md; spec/functional/FR-040-check-composed-values.md |
| FND-002 | medium | AC-5 remains engine-backed through type-stage tags although its positive ordered-query proof/execution obligations are open. Planned matrix status correctly preserves that distinction. | tests/composed_types.rs:193; tests/composed_types.rs:486; spec/model-linking/tests.md |
| FND-003 | low | Complete exact/one-short qualification across every work dimension remains later assurance; zero controls and the independently derived diamond do not establish that entire campaign. | tests/composed_proofs.rs; spec/test-cases/TC-119-check-composed-values.md |

## Correction disposition

The original SR-328 FND-001 high assumed that every constructed
`Unsupported::ValueRepresentation` cause was publicly reachable. Rechecking
actual parser/binder/type admission found otherwise:

- Integer division/remainder/mod and oversized integer/rational literals refuse
  in type admission first. New public tests pin ForbiddenOperator, LiteralDomain
  and RationalNormalization plus the original locus; proof discharge preserves
  UpstreamType. No private TypeReport was manufactured for a test.
- Wrong field bases and unsupported self-read anchors are excluded by existing
  type/binder constructors. Generated symbols use the already validated ClauseId
  grammar; role/representation and materializer-symbol paths retain private
  invariants. No valid public input reaching the claimed missing refusal was found.
- Actual IR representation-construction failures remain distinct and retain
  their diagnostic rather than becoming a fabricated successful representation.

The refused-completeness gap is resolved by leaf, caller and dependency controls.
The unused FamilyControl variant was removed. The refusal-only ordered-query
control now cites AC-4, while full AC-5 remains open. Opaque proof witnesses have
an explicit non-executable contract and a positive native-type separation control.

## Coverage and limits

Claude ran `quire coverage` with the exact worktree scope: **351/359 backed**,
243 criteria and **zero status lies**. Retagging the refusal test did not change
the total because the type tests still cite AC-5. All FR-040 matrix rows stay
Planned. The six inherited unbacked rows concern TC-115 / FR-036-AC-5/6/8,
TC-010 / NFR-005-M-1 and FR-017-AC-2; three unmatched IT-004 tags and twenty
untracked symbols are also inherited. They were not silently marked complete.

No plan bundle was invented for this existing ticket. No new stub, test bypass,
unowned requirement or unmatched test tag was found in the corrections.
The optional semantic-review extension remains declined; ordinary review of
code, test claims and refusal reachability was performed.

The reviewer-authored local gate batch passed: **478 minimal / 494 all-feature
tests**, zero failures and four existing ignored lanes each; both strict Clippy
configurations and formatting passed. Those totals include all three doctests.
Execution provenance and logs are in SR-327. Full FR-040, TC-119 and compiler
#35/#36/#39/#40 remain open.
