---
id: SR-521
title: "Integrity review of the M-6a owner-question rulings"
type: SpecReview
analysis: integrity
scope: "PR #353 diff against main: ADR-011 §2.1, §2.2, §2.4, §5, §6.2, §7.3 and the 2026-09-22 rulings; ADR-012 §2; ADR-013 O-04, O-14, T-3, QC-18; FR-062, FR-065"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

§2.4's claims match QSpec at `2449ceb`:

- `PackageLock` has the seven members listed.
- `IdentityPreimage` has every one of them except `sources`, plus `version`
  and `identity_projection`.
- `dependency_selections` items are `Selection`, whose `DefinitionRef` has the
  constant `digest_domain` `quire.definition.bytes/v1`.
- The value lock's `edition` role is `agent-ix` / `ix:native` / `quire-draft
  1-draft.2`.
- The positive fixtures carry the `quire-edition` and all-`1` edition, the
  `catalog` and all-`3` diagnostics reference, and `required_features =
  ["quire.value.complete/v1"]` with one `available` report entry.

Three things break the internal consistency of the spec:

- The per-item requirement records lost their carrier between E3 and E7.
- The O-04 rewrite gives `declaration` a package-scoping role that FR-322's
  package-local `declaration` cannot play.
- §2.4 contradicts itself on where profile digests come from.

The OQ-3 ruling and the applied §7.3 placement of `NativePackage` and
`runtime` also disagree. ADR-011-OQ-2 declares that disagreement openly, but
FR-065 states the M-6c placement as settled.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The per-item requirement records have no stated carrier from E3 to E7. E3 makes them. The E4 and E5 Proof-metadata cells no longer mention them: E4 carries `bounded_domain`, `model_population` and FR-322's feature-level `capability_report`, and E5 carries the IR table. Yet the E7 input is "IR nodes with the per-item requirement records", and `route` reads "the per-item `Requirements` records made at E3". Under §2.2's column rules, the records are neither Carried nor Dropped at E4, and no text says how CG joins a record (occurrence key, `request_index`) to an IR node. Fix: in the E4 Proof-metadata cell, add "Per-item requirement records are carried in the in-process `CheckedPackage`, not in the v2 bytes". In the E7 input, say the orchestrating driver takes them from that `CheckedPackage` (E4) beside the S5 IR nodes, keyed by occurrence key. Name the wire they cross to CG, the #134 candidate-set wire, or record the gap as a QSpec question. | ADR-011 §2.1 E7 (:259), `route` bullet (:321-322); §2.2 E3, E4, E5 (:356-358) |
| FND-002 | medium | O-04 says "Package scope comes from a node's owner or its FR-322 `declaration`". FR-322's `declaration` is `{qualified_name}`, a package-local path (FR-322:160-172 at 2449ceb), so it carries no package. The same sentence then says a declared record, tuple or enum carries its `declaration` while its `composite_type` node has "no package scope". ADR-011 E3 (:356), ADR-012 §2 Identity (:210) and FR-062 part 1 repeat the owner-or-declaration wording. Fix: "Package scope comes only from an owner (`ModelOwner`, source or definition owner). A `declaration` contributes its package-local qualified name to the content key, not a package." This also settles SR-520 FND-001. | ADR-013 O-04 (:178), O-14 (:365); ADR-011 :356; ADR-012 :210; FR-062 part 1 |
| FND-003 | medium | §2.4 contradicts itself on digest sourcing. It says "Every definition digest QSL writes is read from a QSpec-published accessor", but its `profile_selections` row takes the source header's `profile … digest …` "as written". Profile references are `quire.definition.bytes/v1` `DefinitionRef`s. Fix: say the header's profile digest is written as declared and E3 refuses it unless it equals the accessor's entry for that role, or narrow the sentence to "every digest the source header does not declare". | ADR-011 §2.4 (:495, :502) |
| FND-004 | medium | The OQ-3 ruling bullet says `NativePackage`, `lowering`, `runtime` and IT-010 are "Deleted in the last M-6a change". §6.2 (SEAM-1, the `native_model` row) and §7.3 (the M-6c row) retire `NativePackage` and `runtime` in M-6c. ADR-011-OQ-2 is open on that placement. FR-065 (:181-183, :262-264) states the M-6c placement as settled and does not cite ADR-011-OQ-2, so a QSL-8 implementer reading FR-065 or the ruling gets opposite instructions. Fix: in the OQ-3 bullet, add "As applied, `NativePackage` and `runtime` retire with native `run` in M-6c, pending ADR-011-OQ-2". Cite ADR-011-OQ-2 in FR-065 at both places. | ADR-011 :1204-1212, :1250-1258, §7.3 M-6c row; FR-065:181-183, :262-264 |
| FND-005 | medium | The #216 per-lane paragraph covers only native `run`, which "writes no package bytes and no backend artifact". #216 bans any "library API" producing a checked package outside the spine. `package::NativePackage` survives M-6a, and FR-019 (:31-32) has it owning "complete artifact bytes" of native-linked-package/1. That makes it a library API producing a native package, and the `encoding` and `wire` writer submodules survive with it. Fix: extend the ADR-011 :1044 sentence to the `NativePackage` library constructor and the native-linked-package/1 writer, or state that #216's "checked package" means `quire.checked-package/v2` only. | ADR-011 :1042-1049; FR-019:31-32; SEAM-1 row (:802) |
| FND-006 | low | §5 says "The source header is the lock evidence (§2.4)". §2.4 takes the edition, `definition_selections` and law digests from the QSpec accessor, and `required_features` from E4. The header also supplies `model_selections` "matched to the domain packages admitted at I1", but spine `compile <identity> <revision> <path>` reads no domain package. Fix: say "the source header, with the QSpec accessor, is the lock evidence", and add that E3 refuses a `model` declaration under spine `compile`, as it refuses an `import`, until an operand for domain packages exists. | ADR-011 §5 (:630-634); §2.4 (:497) |
| FND-007 | low | E4 says the v2 `capability_report` has "one `{feature, disposition}` entry per `required_features` entry". FR-322 (:200, 2449ceb) requires a disposition for "every required feature and selected model capability". Fix: add "and each selected model capability". | ADR-011 §2.2 E4 (:357) |
| FND-008 | low | The §5 intro says "The field list and the exit-code values are #225's". The new bullet fixes the operands of `compile` but leaves spine `run`'s operands unstated: package source, `QualifiedName`, argument encoding and object environment. Fix: say both operand shapes are the M-6a shapes that #225 may replace, and give `run`'s operands, or defer both to #225. | ADR-011 §5 (:609, :630-639) |
| FND-009 | low | The E1 row says "Edition read from source", and the header spells `edition "1-draft"`. §2.4 writes edition revision `1-draft.2` from the lock. No text maps the header's edition to the lock's `edition` role. Fix: add to §2.4's `edition` row "matched by identity to the header's `language … edition …` declaration". | ADR-011 §2.2 E1 (:354); §2.4 (:494) |

## Resolution

Fixed in the PR: FND-001 (E4 carries the records in the in-process `CheckedPackage`; E7 says the driver passes them; the wire to CG is raised in "To QSpec"), FND-002 (O-04 and its restatements: scope only from an owner, `declaration` is package-local), FND-003 (E3 refuses a header profile digest that differs from the accessor's), FND-004 (the OQ-3 bullet states the applied M-6c placement, pending ADR-011-OQ-2; FR-065 cites it), FND-005 (the #216 paragraph covers the `NativePackage` API), FND-006 (§5 names both lock sources and refuses `model` and `import`), FND-007 (E4 adds model capability entries), FND-009 (§2.4 edition row maps the header's edition).
Left: FND-008. Spine `run`'s request fields stay #225's.
