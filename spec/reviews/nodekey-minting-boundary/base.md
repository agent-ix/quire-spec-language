---
id: SR-508
title: "NodeKey minting boundary review (ADR-013 O-04, #211)"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, src/value/node.rs, src/value/enumeration.rs, src/value/unit.rs, src/value/model_query.rs"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-508: NodeKey minting boundary review

## Summary

PR #249's `arch-lint api-surface` T12-B rule (ADR-011 T-12, ADR-013 O-04) found
the kernel `NodeKey` constructor called from `src/` outside its documented
"today" caller list. This review verifies the call graph (not the module
names) for every site, to settle which sites are genuinely outside the S3
check stage and which are inside it with an incomplete "today" list.
Evaluated `quire-spec-language@236bee7` (branch point of `task/211-nodekey-
minting`, `origin/main`).

## Verdict

**ACCEPT WITH FINDINGS** — one documentation gap corrected on this branch
(FND-001); one already-tracked, differently-owned violation left untouched
(FND-002), because its correct remediation needs a public-API change gated
behind the #213 S-1/S-2 kernel-extraction lane (ADR-013 T-6), out of this
review's minting-path scope. This is not owner acceptance.

## Method

`NodeKey`'s one hashing constructor is `NodeKey::of` (`src/value/node.rs:53`),
wrapped by the crate-internal helper `node_key_of` (`src/value/node.rs:320`).
Its other constructor, `NodeKey::from_bytes` (`src/value/node.rs:49`), copies
raw bytes with no hashing and no domain check. `grep -rn "node_key_of\|NodeKey::from_bytes"
src/` at 236bee7 finds every call:

- `src/value/node.rs:322` — `node_key_of`'s own body (defines the helper; not
  a caller).
- `src/value/enumeration.rs:117` — `EnumDeclarationPreimage::node_key`.
- `src/value/enumeration.rs:162` — `EnumMemberPreimage::node_key`.
- `src/value/unit.rs:198` — `DimensionPreimage::node_key`.
- `src/value/unit.rs:311` — `UnitPreimage::node_key`.
- `src/value/model_query.rs:108` — `to_object_reference`'s `NodeKey::from_bytes`
  call (a byte reinterpretation, not a digest computation).

For each site, the question is whether it is reached only from a check path
in production, with the preimage digest computed as O-04 requires, or is
genuinely outside the check stage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `value::enumeration` (`enumeration.rs:117,162`, called from `EnumDeclaration::admit`/`admit_member` at `enumeration.rs:181,209`) and `value::unit` (`unit.rs:198,311`, called from `UnitGraph::admit` at `unit.rs:449`) mint I04 node ids by computing a fresh SHA-256 digest over a JCS preimage and comparing it to a caller-supplied key — exactly O-04's own definition of minting, and the same operation `value::expression::check` performs. `grep -rn "EnumDeclaration::admit\|UnitGraph::admit" --include="*.rs" .` at 236bee7 finds every caller: `tests/text_enum_identity.rs`, `tests/equality_matrix.rs`, `tests/quantities.rs`, `tests/collection_algebra.rs`. No `src/` module calls them: `model::checked_dispatch.rs:1019` builds its `PackageDeclarations` with `..PackageDeclarations::default()`, leaving `enums: Vec<EnumBinding>` empty, and no `src/model/intake` module exists yet to populate them either. Issue #118, which landed both files, scoped its exit criteria to "preserve I04/I05 compatibility... without minting a checked package or claiming runtime execution authority" — this module was deliberately built as a standalone identity/admission primitive, not wired into a check-stage caller. Per ADR-011 §6.2, both modules are layer-3 `semantic_value`, and layer 3 as a whole implements S3 (§6.1's layer table, "Stage" column); `value::library` is ADR-011 §1's precedent for a layer-3 module outside the `check` core sub-layer still counting as an S3 "today" implementer. **Fix (this branch):** ADR-011 §1's S3 "today" list named only 3 modules; it is corrected to add `value::enumeration` and `value::unit`, with the explanatory paragraph stating plainly that no check-stage caller exists yet and construction today is test-support only — the same carve-out ADR-011 §1's I1 row already states for a caller-constructed `DomainPackage`. Not a decision to leave enum/unit checking permanently unwired: the family-checker ticket that wires `model::intake`/`check` to call `admit`/`admit_member` for real domain packages remains open, tracked separately from #211. | ADR-011 §1 S3 row, §6.1 layer table, §6.2 `value` non-kernel row; `src/model/checked_dispatch.rs:1019`; #118 |
| FND-002 | medium | `src/value/model_query.rs:108`'s `NodeKey::from_bytes` call reinterprets an `EffectiveId`'s bytes as a `NodeKey` with no digest computed — not a mint in O-04's sense, but a domain-conflating byte transfer ADR-013 O-05 already names and rejects: "the `EffectiveId` ↔ `NodeKey` transfers at `value/model_query.rs:108,123,155` have no canonical role" (ADR-013 §O-05, resolving OBS-018), and O-04's own "Public type" field already decides `NodeKey::from_bytes` (`value/node.rs:49`) is removed. `value::model_query` is ADR-011 §6.2's layer-3 `model` row, not `check`, so this is a real O-04 violation, already correctly documented — nothing in the spec text needed correcting here. The only sound fix is the one ADR-013 already names: retype `ObjectReference::object_type` (`src/value/reference.rs:65`) as `EffectiveId` instead of `NodeKey`, eliminating the transfer. That field is read by `TypeEnvironment::object_type` (`src/value/reference.rs:139,168`) and by `src/value/composite.rs` and `src/value/expression/evaluate.rs`, so retyping it is a crate-wide, public-API-changing move of the kind ADR-013 T-6 assigns to the #213 S-1 kernel edge cuts (X-1) — the serial lane #211's own task brief places out of scope ("keep the change to the minting path and do not refactor around it"). **Left untouched on this branch.** T-12's rule, after the R1 change scoped to #211 (pattern `node_key_of(`, `value::node` exempt), no longer examines `NodeKey::from_bytes` calls at all, so this finding does not block T12-B; it remains T12-C/O-05's open, separately tracked item under #213. | ADR-013 O-04 Public type field, O-05 Conversions/closing note (OBS-018), T-6; `src/value/reference.rs:63-87,139,168` |

## Disposition

- FND-001: resolved on this branch (`spec/decisions/ADR-011-stage-dag-and-
  dependency-architecture.md` §1 S3 row and new explanatory paragraph;
  cross-reference doc comments added at `src/value/enumeration.rs:11-17` and
  `src/value/unit.rs:11-17`). No code behavior changed: `EnumDeclaration::
  admit`, `admit_member` and `UnitGraph::admit` are unchanged, so no new test
  is required for this finding.
- FND-002: escalated, not decided here. Options for the ADR-013/#213 owner:
  (a) leave as-is, tracked under #213 S-1/S-2, and let T12-C continue to
  correctly fail there until #213 lands; (b) pull the `ObjectReference`
  retyping forward as its own ticket ahead of the rest of #213 S-1, if the
  owner wants T12-C green sooner. This review does not choose between them.
