---
id: SR-246
title: "failure-domain review of state-scalar projection"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-034-project-state-scalars.md; spec/test-cases/TC-112-state-scalar-projection.md"
review_set: subset
---
## Summary

Current recheck scope is narrowed to the one applicable analysis for correction
`09c50a5` against integrated baseline `87b35ea`: public versus defensive failure
boundaries, exact context and read identity, and finite alias work in the two
edited spec artifacts. The earlier all-set review of `b789eed` is historical; its
PASS is not the current verdict. No AssuranceProfile document exists.

Public versus defensive boundary. FR-034 now separates three caller-reachable
materialization stops (ContextMismatch, ResourceExhausted, Cancelled) from
Invariant, and states that every missing correspondence after complete validation
is defensive rather than a caller input error. That division is the one the code
enforces: a `ValidatedContext` is constructor-private and is produced only when
no validation diagnostic was observed, so the invariant reasons are unreachable
through the public path. The predecessor paragraph correctly assigns invariant
invocation-input access, `Current` against a pre/post clause, and missing
selected invocations or declared parameters to native linking and runtime
validation, each keeping its own typed cause and original input location instead
of being reclassified. TC-112 requires those predecessor causes and locations to
be compared, and forbids fabricating a validated context to execute a defensive
branch — the correct expectation for a boundary that must not be reachable.

Exact context and read identity. The requirement that an invariant failure retain
the original projected read, with its linked declaration, model digest and
observations, keeps the failure attributable to one exact native location; the
checked-package identity gate and borrowed artifact/object provenance are
unchanged and still prevent cross-request substitution. The caller stops
correctly carry no read, because no particular read caused them. TC-112 pins
completed work and absence of partial inputs for those stops; it pins the read
identity for ContextMismatch and ResourceExhausted but not for Cancelled, a small
gap recorded in SR-253 FND-002 rather than a missing spec constraint.

Finite alias work. The new behavior line bounds candidate search inside the
existing node ceiling — one charge per fresh candidate including collisions, none
for reuse, no added depth — so the search has a stated error at a finite ceiling
instead of relying on catalog finiteness. TC-112 requires the exact ceiling,
exhaustion during search, the one-short whole-package case and a fresh retry, so
the bound is verified at its edge and in the atomic-refusal direction. No new
budget framework is introduced and no existing limit is raised.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No missing failure-domain constraint found in the corrected text; the public/defensive split, read identity and finite alias bound are stated and verified. | FR-034 Inputs/Behavior; TC-112 |
