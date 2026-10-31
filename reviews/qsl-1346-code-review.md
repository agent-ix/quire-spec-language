---
id: SR-1918004
title: 'Code review of PR #1927: admitted lookups by structural wire id (FR-396-AC-10), code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@887f2f0ee; git diff origin/main...887f2f0ee (PR #1927, branch qsl-admitted-lookup-by-wire-id): qsl-replay/src/admitted.rs, qsl-replay/tests/admitted_wire_lookup.rs, spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md, spec/test-cases/TC-1090-the-replay-facade-returns-the-admitted-record-and-tuple-keys.md; ticket QSL-1346'
review_set: subset
date: '2026-10-31'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
---

## Summary

Ticket: QSL-1346. PR: quire-spec-language#1927, head 887f2f0ee. Method: code-review with the rust-review lane folded in. The qsl-replay/src/witness.rs cfg(test) hunk is out of scope per the brief (dropped when the branch merges main after #1926).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| node_key_of_wire, wire_key | qsl-replay/src/admitted.rs:359-401 | examined |
| enum_declaration_by_wire, enum_case_by_wire, composite_declaration_by_wire | qsl-replay/src/admitted.rs:499-564 | examined |
| AdmittedLookupRefusal::MalformedWire | qsl-replay/src/admitted.rs:350-356 | examined |
| wire_node_id (inverse) | qsl-replay/src/spine/bind.rs:575-613 | context_only |
| structural_node_key | qsl-replay/src/spine/bind.rs:617-658 | context_only |
| node_id, admitted_key | qsl-package/src/emit.rs:356-440 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean for major bugs. node_key_of_wire is the exact inverse of the NodeKey to CheckedNodeId serializers (wire_node_id in qsl-replay/src/spine/bind.rs, node_id in qsl-package/src/emit.rs): Name maps to PathSegment::Text, Node maps to PathSegment::Node of the recursively converted key, and package and ordinal pass through unchanged into NodeKey::new. It is iterative with an explicit stack. The three by-wire methods delegate straight to the NodeKey lookups, so Foreign, Unknown and WrongKind are the NodeKey methods' own. A malformed id at any nesting depth surfaces as MalformedWire with the kernel's InvalidNodeKey cause. Non-blocking: the function is a third copy of the same conversion (structural_node_key in the same crate returns Option and drops the cause; admitted_key in qsl-package). The let-else EmptyPath arms on an empty stack cannot be reached.
