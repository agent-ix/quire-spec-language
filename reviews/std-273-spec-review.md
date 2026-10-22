---
id: SR-15920
title: "spec review of STD-273 instance key-context binding (QSL PR #1252)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@42f2b6659ac279be92d1a44375398cb923d7534e; spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md, spec/test-cases/TC-1164-an-instance-key-context-binds-its-population-and-key.md"
review_set: subset
evaluated_revision: "42f2b6659ac279be92d1a44375398cb923d7534e"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1164
    type: reviews
---
# SR-15920: spec review of STD-273 instance key-context binding (QSL PR #1252)

## Summary

Ticket: STD-273 (unblocks QSL-890). PR quire-spec-language#1252, head 42f2b6659 (matches `git ls-remote`), reviewed as a merge with origin/main 39823fe73 (f40dc9d6f). Three files, spec Markdown only. FR-338 gives an `over` parameter's binding in the automaton-state key context the closed value `["instance",population,key]`, FR-125 Instancing names that binding and drops the QSL-890 remaining-work bullet, FR-338-AC-9 and TC-1164 carry the two-population vector.

These claims hold:

- The pair is the one FR-125 Instancing already uses for the obligation identity (ADR-013 O-09) and QSpec FR-361's refuted binding, so the key context, the obligation and the refuted binding now name an instance the same way. The removed bullet described exactly the defect this change closes.
- Universe keys are strings (FR-120 `keys: Vec<String>`) and the declaration identity text is FR-120's RFC 8785 text, so both members are plain JSON strings; the AC-9 shape `[root, profile, "infinite", [[N, ["instance", T, "a"]]]]` matches FR-338's `context` = `[root, profile, activation, binding]` and its `[parameter_node, bound_value]` binding entries.
- The lead default (a QSL-owned tagged array, no QSpec FR-181 change) is sound: FR-338 calls this algebra "a new translation representation decision", and QSpec FR-331 only transports QSL FR-338's octets as base64url. No QSpec file defines the key context. FR-196's probability-product key embeds the FR-338 key array and FR-126 references it; neither carries a vector with an `over` binding value, so no other vector changes.
- The foreign-context refusal follows FR-338's existing refused-key rule (a key whose context differs is a foreign context, rejected `ShapeMismatch` at `ProductState`), so AC-9 can fail: a checker still writing FR-181's reference value, or ignoring the population, gives equal contexts for `P1` and `P2` or admits `P1`'s state under `P2`.
- Ids: TC-1164 and FR-338-AC-9 appear on no other open PR head; no other open PR touches FR-338 or FR-125. No status wording or compat layer. `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` on the merge exits 0 with no diagnostic on a touched file.

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: AC-9 and TC-1164 step 4 refuse the instance array "plus an object type member", but the value is an array, which has elements, not members. One implementer appends a fourth element, another replaces `key` with an object, a third wraps the array in an object; each builds a different negative. Fix: name the exact negatives, for example `["instance", T, "a", "<object type identity text>"]` and `["instance", T, {"type":"text","value":"a"}]`. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:639, spec/test-cases/TC-1164-an-instance-key-context-binds-its-population-and-key.md:29-30 |
| FND-002 | low | minor: the statement says a change of the key object's most-specific type across states leaves the context unchanged, but no vector checks it. A checker that folds the object type into the context passes AC-9 whenever both populations hold the same type. Fix: add a vector whose successor retypes the key's object (a subtype `B` redefining `Counter`) and assert the context octets are unchanged, or drop the clause. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:137-140 |

## Verdict

Mergeable as is; both findings are minor and cheap to fix in one round (FND-001 is a wording fix to AC-9 and TC-1164 step 4). The encoding, its agreement with FR-125 Instancing, the vector shape, the failure modes of AC-9, id uniqueness and the gate are verified. Owner or planner decisions: none required. The lead default (QSL-owned tagged array with plain JSON strings, no QSpec FR-181 change) is consistent with FR-338 and QSpec FR-331 and needs only acknowledgement.
