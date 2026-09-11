---
id: SR-329
title: "Base requirement review of the compiled protocol artifact contract"
type: SpecReview
analysis: base
scope: "FR-042; TC-121; US-004; spec/spec.md; TM-003 (spec/model-linking/tests.md); docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
  - target: ix://agent-ix/quire-spec-language/US-004
    type: references
---

## Summary

Base checklist review of FR-042/TC-121/US-004 and their normative wire contract,
rechecked at 23a5892 against the corrections made after the original review at
51506ed. IDs, cross-references and the six coverage rules still hold. All three
medium interoperability findings are resolved: the contract now publishes the
typed refusal vocabulary, states the inclusive `U` bound, and gives the exact
first-use type indexing order. What remains is two low bookkeeping items and one
narrowed successor to the vocabulary finding.

## Verdict

**CONDITIONAL** — no high or medium finding; three low items, none blocking.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The published refusal vocabulary is explicitly the Rust variant surface, "not a serialized error protocol"; a second implementation can reproduce the classification but has no stable interchange codes for it | docs/compiled-protocol-v1.md:424; src/protocol_artifact/mod.rs:135 | missing-requirement |
| FND-002 | low | TC-117/TC-121 rows are still appended after TC-119/TC-120 in TM-003's L2 table, breaking the checklist's sequential-ID expectation | spec/model-linking/tests.md:110-111 | missing-requirement |
| FND-003 | low | FR-042 still carries no `FR-042-OPT-*`/`FR-042-CON-*` sections and TC-121 states no Type/Priority in the document; both follow existing FR-040/FR-041 and TC-120 precedent and live in TM-003 instead | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:34 | missing-requirement |

## Resolved since the original review

**Typed refusal vocabulary (was medium).** The contract now carries a result-class
table plus the full discriminant lists. They match the code exactly: six `Error`
classes, twenty-six `Invalid` discriminants, six `Unsupported`, thirteen
`Dimension`, and the four `NumberError` forms with their `Decimal`/`Numerator`/
`Denominator` components. Pass precedence is stated normatively — after the seal
and closed-shape checks an unknown wire version refuses as `Unsupported::Wire`
before dependency inventory — and `intake::read` now calls `headers` before
`selected`, with `unknown_wire_is_classified_before_its_foreign_dependency_inventory`
asserting it. FR-042 gained the matching sentence.

**Type indexing order (was medium).** The contract now specifies binder types,
non-null binding-requirement types, then value types in table order, then the
predicate result or the protocol's channel message and compensation attempt
types, assigning an index on first visit before recursing into option/sequence
element types, with unreferenced types refusing. That is exactly the driver order
`locals` → `values` → `temporal` → `body` and `first_type`'s pre-order index
assignment, with the final `type_seen` sweep refusing unreferenced entries.

**Inclusive `U` bound (was medium).** The contract now reads "from zero through
1,048,576 **inclusive**", matching the `> 1_048_576` refusal in reader, writer and
span checks.

## Checklist results

ID format and uniqueness, cross-referencing and link integrity are unchanged and
conform. FR quality is unchanged and specific: Inputs still separate source
inventory, producer, baseline, contract, dependency bytes and admitted producer
views, and still state that source/profile acceptance never comes from a payload
flag. Performance targets remain thirteen named dimensions with defaults, hard
maxima, charge-before-work, clamping and fresh-retry semantics. Security is
unchanged: the seal is external, digest integrity is not authenticity, a resealed
mutant refuses. Error conditions now carry the published discriminant vocabulary
(FND-001 is only about serialized codes).

Coverage (six rules) still holds. TC-121 step 5 gained the negative await cases —
remove the static progress/closure requirement, change its await subject, or
remove its clock-binding dependency, each requiring `Invalid::Binding` — which
matches the reader's new `timeout_authority` check and its three tests.

## Contract fidelity spot checks

The `ix.artifact-ref/3-draft` `linked-package` seal over complete canonical bytes,
absent self-digest, `canonicalIdentity:null`, closed `Number`/`Integer` tagged
objects, CompactFormatter-only spelling, the member-order table and the structural
edge-expansion table remain internally consistent and match the reader. The two
new normative paragraphs also check out against source: the `Origin.Selected`
self-marker paragraph matches `checking/composed/solver/origins.rs:133`, which
emits `Selected { expression: at }` at the expression itself for mixed immutable
provenance; and the await paragraph matches the `Progress`/`Closure` +
`Subject::Control` + `requires`-contains-clock rule the reader enforces.

`quire spec` gate: 398/398 docs grammar-clean, 0 grammar findings; 83/258 criteria
property-extractable (`/tmp/quire-artifact-corrections-spec.log`).
