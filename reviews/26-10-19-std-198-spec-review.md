---
id: SR-13058
title: "Base review of PR #1016: refinement_internals member and the refinement support set"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6f2f80ba5; spec/functional/FR-147 (The profile's internal list, AC-1 to AC-6, Remaining work); spec/functional/FR-135 (Refinement features the build does not check, AC-5); spec/functional/FR-138 References; spec/decisions/ADR-020 RM-9; spec/test-cases/TC-552 steps 5-6; spec/test-cases/TC-540 step 5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-552
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-540
    type: reviews
---

## Summary

Ticket: STD-198. Review set: subset (base, integrity, failure-domain).
The base checklist covers ID hygiene, AC testability, AC-to-TC agreement
and honest QSL-proposed marking over `git diff origin/main...HEAD` only.
New ids (FR-135-AC-5, FR-147-AC-5, FR-147-AC-6) are unique, each is traced
to a TC that already verifies its FR, and the changed files pass
`quire validate`. The findings are an AC that its own TC contradicts, an
undefined TC fixture, a support set that no test can configure, and an
unmarked QSL proposal. ears-conformance was not run: the new SHALL
statements follow the file's existing pattern and the defects are semantic.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-147-AC-5 says "Two units that differ only in this member have different `package_id`s". TC-552 step 5 expects the opposite for three such units: member absent, `[]`, and an object for another refinement give equal `package_id`s. The Reading bullet agrees with the TC: only the resolved `abstract_internal` enters identity, so the raw member does not. As written the AC is false and no implementation can pass both. Fix: say a member that changes some refinement's resolved `abstract_internal` changes the `package_id`, and one that resolves to the same lists leaves it equal. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:118-125,190; spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:30-32,47-49 |
| FND-002 | medium | TC-552 step 5 uses "an object for another refinement", but the FR-147 fixture unit declares only `CasTwice`, and no other refinement is named. The expected equal `package_id` holds only if that object has `"internal": []`. With a non-empty list, a model-subject refinement refuses `malformed-declaration` (FR-147 "Internal abstract steps"), and a protocol-subject refinement changes its own node and so the `package_id`. Name the second refinement and its list in the fixture. | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:30-32,47-49; spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:154-155,190 |
| FND-003 | medium | FR-135-AC-5 and TC-540 step 5 check the same declarations "under a support set that omits `history-row`", one that omits `protocol-abstract-side`, and "the full set". FR-135 calls the set something the build holds, and does not say how a test chooses a set: no parameter, constructor argument or test seam is specified. #970 hard-codes both refusals, so the AC's "with both in the set each checks" half cannot run until both features ship. A test author cannot write step 5 from the spec. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:146-160,178; spec/test-cases/TC-540-s3-checks-a-refinement-declaration-s-subjects-and-state-mapping.md:34-36,51-54 |
| FND-004 | medium | FR-135's new section chooses `unknown_required_feature`/`unsupported-feature` for a feature the build does not yet check. STD-199 item 4 asks QSpec to confirm that code or choose `unimplemented_capability`, but the section is not marked QSL-proposed and does not cite STD-199. FR-147 marks its member this way. FR-138's new reference and ADR-020 are also silent. Mark the support-set section QSL-proposed pending STD-199. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:144-160; spec/functional/FR-138-check-and-compute-history-fields.md:143-145 |

## Verdict

Not mergeable. FND-001 is an AC that contradicts its own TC and the FR body.
FND-002 and FND-003 leave TC-552 step 5 and TC-540 step 5 impossible to
write without guessing. FND-004 is a marking fix. Clean units: FR-147-AC-1
to AC-4 as edited, FR-147-AC-6, ADR-020 RM-9, FR-147 Remaining work and its
STD-199 citation, and TC-552 step 6.
