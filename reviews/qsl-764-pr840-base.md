---
id: SR-9109
title: "Spec review (base) of quire-spec-language PR #840: protocol reference source form and carrier"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ae803014a9c6f6be9733a88ce9d6caedb9eeeae6; spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/functional/FR-092-key-type-parameter-and-declared-nodes.md, spec/test-cases/TC-1021-a-protocol-reference-resolves-by-typed-key-and-emits-one-carrier.md, spec/tests.md"
review_set: subset
---
# Spec review (base) of quire-spec-language PR #840

## Summary

Ticket: QSL-764. Base review of the protocol reference source form (FR-308),
its S3 resolution and refusals (FR-218), its carrier node (FR-092) and
TC-1021. Compared against the Contract IR reader at main e0bee05 (IR-782):
`structural.rs` `protocol_reference_defect`, `mod.rs` `validate_body`,
`validate_protocol_references` and `owner.rs` `validate_owner_schema`; and
against QSpec FR-322 and FR-272 (cited by id only).

Examined:
- FR-092 "Protocol reference nodes", FR-092-AC-16, FR-092-AC-17: carrier
  body, semantic_type, dependencies, occurrence count and role, and the
  forbidden members match the IR reader exactly (examined)
- FR-218 "Protocol references", FR-218-AC-8, FR-218-AC-9 (examined)
- FR-308 "Protocol reference expression", FR-308-AC-6 (examined)
- TC-1021 and its spec/tests.md row; every new AC has a step; ids unique;
  TC-1021 unused on main (examined)
- FR-113 "nothing emits the protocol itself" (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-218-AC-9 and the FR-218 stale bullet refuse a reference whose bound target key no node of the graph carries with `invalid_package`/`stale-node-key`. QSpec FR-272 scopes `stale-node-key` to node-key recomputation. The IR documents `CheckedPackageRefusalCause::StaleNodeKey` (`shared.rs`) as a node whose retained key is not the digest of its own preimage, and its constructor retains the expected key. A carrier whose key matches its own preimage but whose target is absent is not that case. QSpec FR-322 and the IR reader (`validate_protocol_references`) give exactly this condition (a target key absent from the graph) `missing_declaration`/`missing-name` at `/body/target`. The cause is wrong for the condition. Fix: use `missing_declaration`/`missing-name`, naming the bound key, in the FR-218 bullet, AC-9 and TC-1021 step 5. | FR-218:138-142; FR-218-AC-9; TC-1021 step 5 |
| FND-002 | high | FR-092-AC-16 and FR-092-AC-17 require an emitted graph that holds the target protocol node, and the reader admits it. The new FR-092 rule also forbids writing a carrier whose target is not a node written in the same graph. Existing FR-113 says a checked protocol emits no node for itself: "the package holds no node for its name". No requirement in this PR or on main specifies the protocol node's emission: its tag/form (`protocol`/`protocol_clause`), semantic_type or key. FR-218 binds to "that declaration's typed node key", but no key for that declaration is defined. As written, every reference is stale and is refused, so AC-16 and AC-17 cannot pass. Fix: in this PR, specify the target protocol node (tag, form, semantic_type, key rule) in FR-092 with an AC, and amend FR-113's "no node" sentence. | FR-092:405-430; FR-092-AC-16; FR-092-AC-17; FR-113:39-41; FR-218:123-127 |
| FND-003 | medium | FR-218's first refusal bullet says a name that "resolves to no protocol declaration of the unit" SHALL refuse `missing_declaration`/`missing-name`. A name that resolves to a function also resolves to no protocol declaration. The next bullet says the same input SHALL refuse `ill_typed`/`operator-ineligible`. The two SHALLs contradict each other. AC-8 follows the second. Fix: say "resolves to no declaration of the unit". | FR-218:129-136; FR-218-AC-8 |

## Verdict

**FAIL: 3 blocking findings (FND-001, FND-002, FND-003).**

Confirmed against IR:
- Body: exactly `{term:"reference", target}`.
- Occurrences: exactly one, role `expression`.
- Dependencies: exactly the target.
- semantic_type: equal to the target's.
- No declaration, owner, nominal preimage or recursion group.
- The target is local, with no selector.
- The cause strings `missing-name`, `operator-ineligible`, `stale-node-key` and
  `expression-form` all exist in the QSpec catalog and in the IR cause enum.

The duplicate-reference refusal is forced by the contract. The
`quire.structural-node/v1` preimage carries tag, form, semantic_type,
declaration, recursion and body, with no occurrences. Two references to one
target would therefore have one key. IR refuses a carrier with more than one
occurrence. So the two references can neither be two nodes nor share one
carrier.

`protocol` is already a reserved token. The `ProtocolClause` production
starts only in declaration position, so the new `primary` alternative does
not conflict with it. All four changed files pass `quire validate`
(copies, quire 0.36.2).

Recorded, not reported:
- medium: `unsupported_construct`/`expression-form` for a duplicate
  reference does not fit the retained data that QSpec FR-272 gives that
  cause, which is profile-based.
- medium: no FR names a host position where a protocol-typed `primary`
  checks. FR-308 adds no consumer, so the TC fixture must choose one, for
  example a `let`.
- low: FR-218-AC-8 and AC-9 sit before AC-7 in the table.
- low: AC-8 does not assert that the duplicate refusal names the first
  reference.
- low: the IR checks the target's tag only, so any `protocol` form is a
  valid target. QSL narrows this to a protocol declaration, which is
  compatible.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 26e53f967 |
| FND-002 | fixed | 26e53f967 |
| FND-003 | fixed | 26e53f967 |

Disposition round 1, at 26e53f967945ec66aa288c24fe69ac33b6a43fa4. The new
FR-092 section "The protocol node a reference targets" and FR-092-AC-18 were
checked against IR main e0bee05 and QSpec FR-322:
- Tag/form `protocol`/`protocol_clause`: `validate_protocol_references`
  accepts any local node with the `protocol` tag and an equal semantic_type.
- Body: the `quire.op.protocol.control` application with the
  `protocol_profile` law and the `profile_operator` member (FR-322-AC-22).
- Boolean semantic_type.
- Key: `quire.application-node/v1`, whose preimage carries `declaration` and
  no owner.
- One `declaration` occurrence.

This matches QSpec's positive-all-families protocol node. FR-113, FR-218,
FR-092 and TC-1021 agree with each other. AC ids are unique. All four
changed files pass `quire validate` on copies. `qsl-package` emits no
`protocol_clause` node today, which is target design and not a
contradiction.

Recorded, not reported:
- medium: "`declaration` and `owner` as FR-322 gives a declared clause node"
  is correct but indirect. FR-322 gives an application-keyed node no
  `owner`, while FR-092's other declared nodes carry the unit's owner.
  Stating "no `owner`" would prevent that mix-up. AC-18's reader admission
  catches it either way, because the IR refuses an owner on an
  application-body node.
- medium: AC-18 does not pin the `profile_operator` value or the
  application's arguments.
