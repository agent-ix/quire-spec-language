---
id: SR-1951003
title: "Spec review (base) of PR #1951 (QSL-1315): TC-160 step 8"
type: SpecReview
analysis: base
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1951 git diff origin/main...HEAD, spec/test-cases/TC-160-checked-family-contract-shape.md step 8 and its expected result, read against spec/functional/FR-062-implement-checked-family-contract.md FR-062-AC-9; ticket QSL-1315"
review_set: base
relationships:
- target: ix://agent-ix/quire-spec-language/TC-160
  type: reviews
---
# SR-1951003: spec review (base) of PR #1951 (QSL-1315)

## Summary

Ticket: QSL-1315. The only spec file the PR changes is TC-160. Step 8 and its
expected result move the FR-062-AC-9 scenario from a definition-owned `metre`
unit, which no producer can reach any more, to a union `Shape` that "the IR in
use cannot decode". I ran the base checklist only, with no optional analysis
lenses; no `AssuranceProfile` document exists under `spec/`. `quire validate`
on TC-160 passes. A negative probe (a copy with a lowercase `id`) was reported,
so the validator really read the file.

One finding: step 8 tells the reader to emit with `emit_checked`, but
`emit_checked`'s IR decodes `union`, so the step cannot produce the omission it
describes. The bound test injects a decoder through the private
`emit_package_decoding` seam instead.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| TC-160 step 8 (procedure) | spec/test-cases/TC-160-checked-family-contract-shape.md:70-74 | examined |
| TC-160 step 8 (expected result) | spec/test-cases/TC-160-checked-family-contract-shape.md:136-140 | examined |
| FR-062-AC-9 | spec/functional/FR-062-implement-checked-family-contract.md:375 | context_only |
| emit_checked / emit_package_decoding | qsl-package/src/emit.rs:1721-1726, 2357-2363 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Step 8 cannot be run as written. It says to emit, with `emit_checked`, a package whose union `Shape` "the IR in use cannot decode". `emit_checked` always runs the real IR vocabulary, and that vocabulary decodes `composite_type`/`union`, so `emit_checked` omits nothing for this package. The bound test `a_function_naming_an_omitted_node_is_omitted_and_the_other_is_written` gets the omission only by passing a decoder that refuses `union` to the private `emit_package_decoding`. FR-062-AC-9 also names `emit_checked`. Fix one of two ways: state the decoder seam in step 8 (and in AC-9's first sentence), or rebuild step 8 on an omission that `emit_checked` really makes, such as an enum whose foreign owner the lock does not select (`UnlockedOwner`). | spec/test-cases/TC-160-checked-family-contract-shape.md:70-74; spec/functional/FR-062-implement-checked-family-contract.md:375; qsl-package/src/emit/tests/union_nodes.rs:233-249 |

## Verdict

**CHANGES REQUESTED** (FND-001).

Otherwise the edit is sound:
- The removed scenario depended on a `DefinitionOwner` node owner, which FR-322 rules out and no producer reaches. Replacing it was right.
- The new expected result is consistent with step 8's procedure: union omitted with `UnsupportedForm`, `area` and its path omitted with `NamesOmittedNode`, `t` written, and an I2 export of `t` only.
- The step introduces no hash, pin or tracking record.

Base-checklist ID, link and cross-reference checks on TC-160 pass.
