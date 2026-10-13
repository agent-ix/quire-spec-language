---
id: TC-985
title: "Record-valued model fields and observations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-985: Record-valued model fields and observations

## Description

Qualify FR-056-AC-21/22, FR-103-AC-4/6 and FR-106-AC-14/15/16/17 over
the same selected Semantic IR domain package and checked spine package.

## Preconditions

- The selected domain package declares record value types `Street { name:
  Integer }`, `Address { street?: Street }`, and `PostalAddress: Address {
  zip: Integer }`, object type `Account { address: Address }`, and an
  operation on `Account` with record-typed parameter and result and a frame
  capable of naming `Account.address` and `Address.street` separately.
- A second record `Node { next?: Node }` supplies a finite recursive type.
  Every record has the exact FR-208 meaning id; each declaration and member
  retains its IR node identity. The QSpec FR-322 record-node schemas are
  selected for checked-package emission.
- Snapshot and invocation documents use FR-106's `quire.state.snapshot/v1`
  and `quire.state.invocation/v1` forms with selected `sha256-jcs` digests.

## Test Procedure

1. Admit and assemble the selected package. Inspect `Address` and
   `PostalAddress` exports, effective fields, subtype conformance, the
   `Account.address` field, optional and ordered bounded record fields,
   and record-typed operation parameter and result. Check
   `deref(account).address.street` and emit the checked package.
2. Mutate the domain package separately to give a record an identity field,
   an operation, a field `typeRef` naming each prohibited business meaning,
   and an optional `Node.next` recursion; admit each under the same limits.
3. Admit a current snapshot containing a `PostalAddress` record in
   `Account.address` with a present nested `Street`, then reorder JSON
   members. Repeat with that record in an operation parameter and result.
4. Mutate the record value separately to omit `street`, add an unknown
   field, select an object type, select an unrelated record, select a missing
   type, and replace the record tag by a reference tag. Place each defect
   inside one sequence element and inside a parameter or result.
5. Admit a finite `Node` chain of 10,000 present `next` fields with limits
   large enough for its bytes and values. Repeat at the exact value bound
   and with one further value. Evaluate an invariant false at one nested
   record value.
6. Evaluate a postcondition invocation changing `Account.address.street`
   with a frame grant for the declared `Account.address` field, then remove
   that grant. Replace `PostalAddress` by `Address` in one post snapshot
   without changing any shared field, again with and without the grant.

## Expected Results

- Step 1 admits value-typed record declarations and members, preserves
  effective fields and conformance, and emits a self-typed, model-owned
  `record_value_type` node with family `record`. The checked field and
  operation types name that node, never `Reference<Address>`.
- Step 2 refuses the identity field, operation and wrong-meaning `typeRef`
  with FR-208/FR-154's respective named causes before assembly. The finite
  recursive declaration admits with its optional edge.
- Step 3 admits the same typed observation and digest for member-order
  variants; each record retains its most-specific type and exact nested
  FR-207 path, under object and invocation roots respectively.
- Step 4 refuses with `invalid_runtime_input` and respectively
  `missing-member`, `unknown-member` or `wrong-value-kind`, retaining the
  nested field, option-value and index path and the invocation root when
  applicable.
- Step 5 admits finite deep values without a depth refusal. An exact
  value-count bound admits; one further value returns `resource_exhausted`/
  `insufficient-next-charge` naming the bound, prior count and next path.
  The false invariant is a located logical violation after admission.
- Step 6 admits with the `Account.address` grant and refuses
  `frame_violation`/`unauthorized-change` naming that declared object field
  without it, for both the nested value change and the type replacement.

## Traceability

The expected results verify FR-056-AC-21/22, FR-103-AC-4/6 and
FR-106-AC-14/15/16/17. The earlier ConfigVersion operation cases remain
in TC-458.
