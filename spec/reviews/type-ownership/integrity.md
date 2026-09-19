---
id: SR-484
title: "Integrity review of ADR-013 canonical type, package and conversion ownership"
type: SpecReview
analysis: integrity
scope: "spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-484: Integrity review of ADR-013 canonical type, package and conversion ownership

## Summary

Reviewed commit 660aa25 on `task/211-type-ownership`
(agent-ix/quire-spec-language): ADR-013 and its index row in `spec/spec.md`.
The working tree later added one paragraph that scopes the local id prefixes;
it changes no finding below. ADR-013 is the Layer 1 design for #211. This pass
checks four things:

- completeness against every #211 object, every "Required design" field and
  every acceptance bullet;
- that every routed ADR-010 item is decided exactly once;
- that the ids, cross-references and counts agree;
- that the record agrees with accepted QSpec AD-016 and the QSpec FRs it cites.

What holds:

- **Coverage.** Every #211 object has an O- section: domain, package,
  declaration, member, occurrence, frame, clause and witness identities; names;
  locations; values; types; typestate; the six outcome categories;
  capabilities; proof modes; limits; versions; digests; proof results;
  counterexamples; and replay requests and results.
- **Routed items.** §9 decides each of the 17 DA items (DA-01..DA-18 without
  DA-11) and each of the 16 routed OBS findings exactly once. The Context
  counts match ADR-010 §9.2 and §9.3.
- **Program rules.** R-08 and §5 state one version per contract with explicit
  refusal. No shim, adapter or legacy reader is designed. No rename is
  proposed. The Mermaid labels contain no `;`. The #229 → #213 → #185
  capability chain and the #222 design / #213 implementation split for bounds
  are recorded as the program rules require.
- **Verified claims.** Spot-checked claims hold: the FR-322 `OperationMember`
  union, the revision `1-draft.4` catalog vendored under
  `resources/complete-value`, the 10 `KaniOutcomeKind` kinds, and the IR PR
  #139 head `417ec86`.

The two blocking defects are in the identity layer that #211 exists to fix:

- The source-occurrence identity (O-07) contradicts the FR-322 `source_map`
  key that the record names as its authority.
- The counterexample packet (O-25) lacks inputs that the replay request (O-26)
  needs. Deterministic replay, a #211 acceptance bullet, is therefore not
  shown.

Medium findings:

- Three decisions depart from accepted AD-016 text, while Context says AD-016
  is not reopened.
- One digest domain the record relies on is missing from FR-201.
- The outcome category map is incomplete.
- Several objects lack required-design fields.
- §4 and §7 disagree with §3.
- Some work is assigned to tickets whose scope excludes it.
- OBS-026 is deferred to owner questions rather than decided.

Verdict: REVISE (FND-001 and FND-002 are high and blocking; FND-003 to FND-012
are medium and should be fixed before the #212 gate).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Source-occurrence identity contradicts its authority. O-07 defines an occurrence as (checked node id, source document identity, byte span) and its equality as lexical over (node id, source digest, byte start, byte end). O-07 names FR-322 `source_map` as the serialized authority. But FR-322 keys an occurrence by (`node_id`, `role`, `ordinal`), where role is one of `declaration`, `type`, `expression`, `anchor`, `claim` or `generated`. One occurrence may name several ordered, non-overlapping regions. Each region sits under a source-document identity, revision and raw-byte digest (`schema.json` `SourceMapEntry` requires `node_id`, `role`, `ordinal`, `regions`). Under O-07's key, two occurrences of one node that share a span, such as a `declaration` and a `type` role, would be equal. A multi-region occurrence has no key at all. O-12 has the same defect: it types spans as `LocatedSpan` (`QSL:src/source.rs:47`), which is start and end `Position`s with no document identity. O-12's equality is (source digest, byte range) without the revision. Fix: make O-07's public type and equality the FR-322 key (`node_id`, `role`, `ordinal`), with an ordered list of regions, each under (document identity, revision, raw-byte digest). Make O-12's canonical span carry the document identity, and state that `LocatedSpan` is a source-stage helper that gains that identity when it enters the node-keyed map. | ADR-013 O-07, O-12, C-14 · QSpec FR-322 `source_map` · `proposals/checked-package-v2/schema.json` `SourceMapEntry` |
| FND-002 | high | The replay inputs do not close. O-26 selects the function by checked node id and recompiles "the locked source", and refuses "a selection naming no function node". The packet members in O-25 are the obligation identity, clause node id, `package_id` and contract version, backend identity and tool pin, and the witness. None of them names the function node to select, and none names the source document or lock to recompile. A `package_id` is a digest, and FR-322 excludes source bytes from its preimage, so the executor cannot find the source from it. C-12 claims the request "carries every O-25 member", which is not enough to build it. #231's scope also requires the envelope to carry occurrence identity, source provenance, semantic profile, proof bounds and trace position. The O-25 list omits all of these. So the #211 acceptance bullet "Witness/replay types preserve enough stable identity and values for deterministic native replay" is not shown. Fix: add to the O-25 packet list and the O-26 request (1) the selected function node id, or a stated derivation from the clause node id through the checked package; (2) the FR-322 `lock` source-document identity and digest, and how the executor obtains the bytes; (3) the semantic profile selections; (4) the declared per-argument bound; (5) the occurrence identity. Then update C-12. | ADR-013 O-25, O-26, C-12, C-13 · #211 Acceptance · #231 Scope · QSpec FR-322 `identity_preimage`, `lock` |
| FND-003 | medium | Context says AD-016 is built on and "not reopened", but three decisions contradict accepted AD-016 text and depend on amendments OQ-3 has not obtained. (a) O-05 and Q209-4 put `EffectiveId` and the reference identities in the kernel. AD-016's kernel row holds "exactly the types listed in this row plus `Undefined`". (b) O-25 stores only `transcript`. The AD-016 Replay-ownership row lists five `Witness` fields. (c) O-11 and O-26 select by node id. AD-016 arrow 7 fixes `CheckedPackage::call(&self, function: &str, …)`. The record states these as decided and says nothing about what holds if OQ-3 is refused. Fix: reword Context to say that ADR-013 proposes three AD-016 amendments (OQ-3). Mark O-05, O-25 and O-26 as contingent on OQ-3 at the #212 gate, and state which text binds until the amendment lands. | ADR-013 Context, O-05, O-11, O-25, O-26, §8 OQ-3 · QSpec AD-016 Shared-type strategy, Replay ownership, Arrow 7 |
| FND-004 | medium | The effective-declaration domain is not in FR-201. O-05 types `EffectiveId` in `quire.model.effective-declaration/v1`. R-04 and §2 compare identities "under the same FR-201 domain". O-18 makes the digest record's domain an "FR-201 domain (closed enum)". Alternatives says "FR-201 keeps the two domains distinct". FR-201's closed vocabulary does not list `quire.model.effective-declaration/v1`, `quire.model.effective-view/v1` or `quire.model.object-universe/v1`. These are defined only by `proposals/checked-package-v2/model-effective-declaration.schema.json`. As written, O-18's closed enum cannot represent `EffectiveId`'s domain, and the Alternatives argument rests on a false premise. Fix: add an owner question for a QSpec FR-201 amendment that adds the three model domains, or name the model-effective-declaration schema as their authority. Widen O-18's enum to match, and correct the Alternatives sentence. | ADR-013 R-04, §2, O-05, O-18, Alternatives · QSpec FR-201 Values · `proposals/checked-package-v2/README.md` |
| FND-005 | medium | The O-16 category map is not total. (a) `Undefined`, a separate kernel outcome that FR-323-AC-1 lists as its own disposition, has no category row. (b) `Inconclusive` sits under "internal failure", but FR-331 `inconclusive` and AD-016's replay-disagreement `inconclusive` are not internal failures. (c) The replay-disagreement result has no category. (d) Three proof cells say "per the IR map", so the record decides no category for `Refused`, `InvalidInput`, `IncompleteInput`, `Unavailable` and `Inconclusive`. (e) FR-331 `tested` and `failed` appear in no row. The claim "no category collapses … through any conversion" therefore cannot be checked. Fix: add an `undefined` row and an `inconclusive` row. Place every `KaniOutcomeKind` and every FR-331 result in exactly one category. Leave only the catalog-code assignment `OPEN — decided in WP9`. | ADR-013 O-16 · QSpec FR-323-AC-1, FR-331 `results`, AD-016 Terminal-disposition rule, Arrow 7 |
| FND-006 | medium | Some O- sections omit required-design fields that #211 requires for each object. O-16 has no validation-and-diagnostics row and no equality kind. O-19 has no public-type invariants, validation or equality for the capability value. O-21 has no validation or equality. O-22 has no public type, conversion or validation row. O-23 has no public type, validation or equality. O-25 has no serialized-authority or version row and no validation row. Fix: add the missing rows, or state per object why a field does not apply (for example, "not an identity"). | ADR-013 O-16, O-19, O-21, O-22, O-23, O-25 · #211 Required design |
| FND-007 | medium | O-25 never cites QSpec's normative counterexample wire. FR-331 defines `counterexamples` as "canonical assignments/traces plus originating run identity", and FR-331-AC-3 requires every returned counterexample to replay through the native owner. O-25 cites only IR `CounterexamplePacket`, IR PR #139 and #231. Its envelope invariant stores the transcript only. The record never says how that envelope serializes to, or is derived for, the FR-331 canonical assignment form. Under R-02, a missing member there is a QSpec change. Fix: name FR-331 `counterexamples` as the serialized authority. State that canonical assignments are derived from the transcript by the C-11 decode, or raise the gap as a QSpec question. | ADR-013 O-25, R-02 · QSpec FR-331 `counterexamples`, FR-331-AC-3 |
| FND-008 | medium | §4 omits conversions that §3 names between layer-owned representations, which breaks R-03 ("keep a local representation only where §4 names the conversion and its test"). Missing: IR `ClauseKind` → CG obligation kind (O-10); clause node id → CG `KaniObligationIdentity` obligation id (O-09); checked member → v2 `OperationMember` (O-06); body span → document span (O-12); kernel `Value` → finite harness domain (O-13); authored bound → proof bound by CG negotiation (O-21, "the only derivation"); `DomainPackageRef` → v2 lock → IR `CheckedDomainPackageRef` (O-01). Fix: add a C- row with owner, loss rule and test for each, or narrow R-03 to state which conversions §4 need not list. | ADR-013 R-03, §4, O-01, O-06, O-09, O-10, O-12, O-13, O-21 |
| FND-009 | medium | The §7 #213 row disagrees with §3. O-01, O-03, O-07, O-08, O-09 (clause id), O-10, O-11, O-14, O-22 (QSL readers) and O-23 (QSL literals) each name #213 as implementing ticket. The §7 #213 row lists none of them. O-03's implementing ticket is #213, yet §7 lists O-03 only under "#131 / QSL PR #200". O-16 and O-22 also name #231, and the §7 #231 row omits both. Fix: build §7 from the Implementing-ticket rows so each O- id appears under every ticket that §3 names. | ADR-013 §7, O-01, O-03, O-07–O-11, O-14, O-16, O-22, O-23 |
| FND-010 | medium | Some work is assigned to tickets whose scope does not cover it, and some has no ticket. (a) §7 assigns FR-322 code completeness (OBS-035) to "IR (#137, PR #139)". IR #137 is "FR-031-AC-3 is verified by a test that discards the witness", and PR #139 adds the `Witness` type. Neither covers `CheckedPackageRefusalCode` completeness, so OBS-035 has no implementing ticket. (b) O-25 sends missing packet members to "IR work under #231 and IR #137". #137 is a test-honesty issue. (c) O-26 and C-13 attribute the executor's recompile, `package_id` check and node-id selection to #231. #231's non-goals exclude "replay execution". (d) O-23 says "CG removes its own literals" (OBS-034) with no ticket. Fix: name or open an IR ticket for FR-322 code completeness and a CG ticket for OBS-034. Assign the executor-side behavior to the ticket that owns replay execution (#217 or a #209-decided owner), and keep #231 for the request type. | ADR-013 §7, O-17, O-23, O-25, O-26, C-13 · IR #137 · #231 Non-goals |
| FND-011 | medium | OBS-026 is deferred, not decided, and its owner is inconsistent. §9 decides OBS-026 as "`native-run-result/2` is QSpec-owned, QSL-produced (#186); OQ-1 and OQ-2 open". The substance (which record is the replay result, and whether `/1` stays) is sent to owner questions. Those are not the legitimate #209 or #210 deferrals. O-27 names #231 as implementer and says #186 "adds only its state-specific payload". But #186 is "State forall separating-witness channel and native-run-result/2 serializer". Its exit criterion, "`/1` output unchanged for cases without a witness", already answers part of OQ-1. Fix: decide OBS-026 in O-27. Name the `/2` serializer's implementing ticket consistently (#186 builds the serializer on #231's carrier), and cite #186's exit criterion in OQ-1 or close OQ-1 from it. | ADR-013 §9 OBS-026, O-27, §8 OQ-1, OQ-2 · #186 |
| FND-012 | medium | The O-20 code owner is ambiguous. Owner says the vocabulary is decided in #210 with #222 and the mode is settled by CG. Public type is "CG's disposition plus the declared finite domain", a CG type. But the implementing ticket is #213, a QSL ticket, for "the typed request and bound representation". The equality "lexical on the mode value" names a value the record does not define. Fix: name the QSL type #213 builds (the typed proof-mode request) and its invariants separately from the CG disposition it maps to. Give the equality of the type O-20 owns. | ADR-013 O-20 · #213, #222 |
| FND-013 | low | R-06 and O-11 are stated more broadly than FR-322 allows. They say no consumer resolves a name after the check stage, and that no name → identity lookup exists. FR-322 `declaration` says a consumer resolves a qualified reference `a::Name` byte for byte against a locked package's `qualified_name`, and that library exports resolve only through this member. O-11's serialized authority also omits FR-322 `declaration.qualified_name`. Fix: carve out cross-package library-import resolution against FR-322 `declaration` (done by the importing package's checker), and cite that member in O-11. | ADR-013 R-06, O-11 · QSpec FR-322 `declaration` |
| FND-014 | low | Some ADR-010 DA members are not placed. DA-15 lists `digest::ByteDigest` (`QSL:src/digest.rs:8`, algorithm-prefixed text), and O-18 neither folds it nor marks it lane-private. DA-13 lists native `Source`/`Span` and `LosslessCst` spans, which appear in neither O-12 nor §6. O-16 calls simulation `Outcome` lane-private, but the §6 Outcomes row omits it. Fix: dispose of each in O-18, O-12 or §6. | ADR-013 O-12, O-16, O-18, §6 · ADR-010 §5 DA-13, DA-15 |
| FND-015 | low | The NodeKey placement and C-02 minting are not reconciled with the sources. O-04 puts `NodeKey` in `quire-exact`, citing AD-016's kernel row. AD-016's own Shared-type row "Semantic identities: `DeclarationKey`, checked node id" says "Layer-owned, QSL `model::key`". The record picks one reading without noting the conflict. O-03 and C-02 mint the `NodeKey` for a model declaration "with `ModelOwner{identity, node}`". In `node-identity-preimage.schema.json`, `ModelOwner` appears only as the `Owner` of enum, dimension and unit nominal preimages. Fix: note the AD-016 row conflict (add it to OQ-3), and state the preimage used for model declarations that are not nominal. | ADR-013 O-03, O-04, C-02 · QSpec AD-016 Shared-type strategy · `node-identity-preimage.schema.json` |
| FND-016 | low | Secondary-#211 items are not traced. ADR-010 §9.2 gives #211 secondary ownership of OBS-001, OBS-031, OBS-037, OBS-039 and OBS-041. ADR-013 addresses OBS-031 (O-23) and, implicitly, OBS-037 (R-10), but §9 records none of them. OBS-041's duplicate quire-rs revision in the lock is not addressed. The third part of OBS-034 (CG `assurance/pins.json` wrongly states there is no Kani code) is not decided. Fix: add a secondary-items table to §9, and decide the OBS-041 and OBS-034 residues or route them. | ADR-013 §9, O-23 · ADR-010 §9.2 |
| FND-017 | low | Some equality kinds and test citations are wrong. O-01 calls equality over (`identity`, `version`, `digest_domain`, `digest`) "normalized". By §2, a tuple of strings plus a digest compares lexically or by declaration. O-26 calls FR-323 request identity "normalized", but FR-323 defines no digest for it. C-16 cites "FR-201 vectors", but FR-201 has no vectors (AC-1 to AC-4 are Inspection or Analysis). Fix: use lexical or declared for O-01, state O-26's comparison as componentwise, and cite a real test for C-16. | ADR-013 §2, O-01, O-26, C-16 · QSpec FR-201, FR-321, FR-323 |
| FND-018 | low | O-23 decides OBS-022 with an alternative: the literals are "derived from the lock or checked equal to it by a test". That leaves two options, not one decision. Fix: pick one. | ADR-013 O-23, §9 OBS-022 |
| FND-019 | low | O-21's four bound kinds omit the kernel bound types that AD-016 places in `quire-exact`: `BoundedInteger`, `CardinalityBound` and `BoundViolation`. Arrow 3 passes these as typed bounds. Fix: place them in one kind (evaluation or authored), or add them to the table. | ADR-013 O-21 · QSpec AD-016 Shared-type strategy, Arrow 3 |

## Method

- **#211 checklist.** Read `gh issue view 211`, then mapped each item to an O-
  section:
  - domain → O-01; package → O-02; declaration → O-03 and O-04;
    member → O-06; occurrence → O-07; frame → O-08; clause → O-09 and O-10;
    witness → O-25;
  - names → O-11; locations → O-12; values → O-13; types → O-14;
  - typestate → O-15 (linked form deferred to #209, Q209-2);
    outcomes → O-16 and O-17;
  - capabilities → O-19; proof modes → O-20; limits → O-21;
  - versions and digests → O-22, O-23 and O-18;
  - proof results, counterexamples and replay → O-24 to O-27.

  Each section was checked against the six "Required design" fields (FND-006).
  Each acceptance bullet was checked: duplicates (§6, FND-014); totality
  (R-07, §4, FND-008); display-name identity (R-05, R-06, FND-013); replay
  sufficiency (FND-002); pins versus heads (O-23). The process bullet
  (`/specify`, `/spec-review all`) is outside the document.
- **Routed items.** Compared ADR-013 §9 with ADR-010 §9.2 (primary #211:
  OBS-005, 006, 017–027, 032, 034, 035 = 16) and §9.3 (DA-01..DA-18 without
  DA-11 = 17). Each appears in exactly one §9 row. No extra or missing ids.
- **Ids and references.** R-01..R-10, O-01..O-27, C-01..C-16, Q209-1..7,
  Q210-1..4 and OQ-1..3 are sequential and unique. Every in-text O-, C-, Q- and
  OQ- reference resolves. §7 was cross-checked against the Implementing-ticket
  rows (FND-009). Ticket titles were checked with `gh issue view` for #131,
  #185, #186, #209, #210, #212, #213, #215, #222, #226, #229 and #231, and for
  IR #137. IR PR #139's head is `417ec869` (open).
- **QSpec sources** (`git -C quire-specification show origin/main:`):
  - AD-016: arrows 1–7, Replay ownership, Shared-type strategy, Owner decisions.
  - FR-201, FR-321, FR-322, FR-323, FR-331, FR-351 and FR-352.
  - `proposals/checked-package-v2/` README, `schema.json`
    (`SourceMapEntry`), `node-identity-preimage.schema.json`
    (`OperationMember`, `ModelOwner`, `QualifiedName`) and the
    model-effective-declaration vectors.
- **QSL code spot-checks** in the worktree: `ByteDigest` (`src/digest.rs:8`),
  `LocatedSpan` (`src/source.rs:47`), `EffectiveId` (`src/model/key.rs:157`),
  `NodeKey` (`src/value/node.rs:22`) and `DomainPackageRef`
  (`src/model/domain_package.rs:359`). Also checked: the vendored catalog
  revision `1-draft.4`, and the `1-draft.3` claim at
  `src/complete/diagnostic.rs:13`.
- **Program rules.** No compatibility layer, shim, fallback or migration path
  is designed. The AD-016 kernel fallback is correctly recorded as inactive.
  §5 regenerates rather than reads old versions. No rename is proposed.
  Mermaid labels are free of `;`. Deferrals are to #209 or #210, except
  OBS-026 (FND-011).
- **Not verified.** RT, CG and FCD code claims; IR code beyond the PR #139 head.
