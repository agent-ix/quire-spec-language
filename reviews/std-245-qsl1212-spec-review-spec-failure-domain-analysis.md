---
id: SR-15460
title: "failure-domain review of STD-245 refusal order and inherited frame_operation (QSL PR #1212)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@4fabd79f984475fc42ba12336433f44d432d6b91; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/functional/FR-071-implement-typed-replay-request.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-116-replay-a-frame-counterexample.md, spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md, spec/test-cases/TC-183-witness-envelope-missing-member-refusal.md, spec/test-cases/TC-185-replay-request-exact-members-round-trip.md, spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md, spec/test-cases/TC-515-replay-a-frame-counterexample.md, spec/test-cases/TC-516-locate-a-function-call-site.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-15460: failure-domain review of STD-245 refusal order and inherited frame_operation (QSL PR #1212)

## Summary

Ticket: STD-245. Examined: FR-070's reconstruction refusals; FR-071's decode refusals before the
byte provision; FR-098's and FR-116's pre-recompile refusals and their order; FR-116's
`frame_operation` resolution against QSpec FR-342 and FR-323; and FR-121's declaring-object rule for the
inherited case against AD-006's original/effective declaration keys and FR-342-AC-4. The FR-121-AC-23
example (`Config::Sub::attemptUpdate` gives `{Config::ConfigVersion, attemptUpdate}`) is correct. FR-342
anchors an operation that is only inherited at its declaring type and refuses `(Sub, op)` with
`malformed-declaration`, which FR-116-AC-8 tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | major: FR-116 has three pre-recompile refusals with no stated order: the target kind, the envelope/request target mismatch, and the existing clause_node/occurrence consistency check. FR-116-AC-8's `function` case over a `frame_operation` envelope meets both the kind rule and the mismatch rule, and the AC expects `invalid-value`, but no SHALL selects it. Their position relative to FR-098's byte-provision rule is also unstated, and AC-8 varies only a source that does not compile. State the order (and align FR-122, SR-15456 FND-001), and add a case with a missing provision entry. | spec/functional/FR-116-replay-a-frame-counterexample.md:151 |
| FND-002 | medium | major: the declaring object is the type "whose own declaration of the operation the selected type's effective view resolves to, found through that view's AD-006 effective declaration and its original declaration keys". For a redefined operation (FR-103-AC-10: `Sub.size` redefines `ConfigVersion/size`), the effective declaration has two original keys: Sub's redefinition and the redefined ConfigVersion member. The rule does not say which one names `object`. QSpec FR-342-AC-4 resolves `(Sub, "size")` to Sub's redefinition, so a rule that walks the redefinition to its root returns `{ConfigVersion, size}`. That pair selects another anchor and frame, and the replay then refuses as stale. FR-121-AC-23 covers only pure inheritance. State that a redefinition's declaring object is the redefining type, and add an AC and TC-516 case (`Config::Sub::size` gives `{Config::Sub, size}`). | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:159-169 |
| FND-003 | low | minor: FR-098's non-function refusal (`invalid_runtime_input`/`invalid-value` at `/replay/frame_operation`) has the same code and pointer as FR-323 rule 1's refusal of a request carrying both `function` and `frame_operation`. A diagnostic cannot tell "two targets" from "frame target given to `replay`". The lead default is sound, but the refusal should carry a distinguishing detail (for example the entry name, `replay`, or the expected member), stated in FR-098, FR-116 and FR-122. | spec/functional/FR-098-execute-a-replay-request.md:463 |

## Verdict

Not mergeable: FND-001 and FND-002 (major).

## Dispositions

Round 1, reviewed QSL #1212 at 373ddf489.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | fixed at 373ddf489 (three ordered checks after decode and byte provision; AC-8 precedence and missing-provision cases) |
| FND-002 | fixed | fixed at 373ddf489 (redefining type is the declaring object; FR-121-AC-24, TC-516 step 24) |
| FND-003 | fixed | fixed at 373ddf489 (wrong-entry target refuses unknown-member, a cause in QSpec FR-272's closed invalid_runtime_input row, distinct from FR-323's two-target invalid-value) |
