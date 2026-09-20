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

**ACCEPT WITH FINDINGS**, revised per owner rulings R6–R9 (Agent A,
2026-09-19). `value::enumeration` and `value::unit` are not S3 implementers
today; they are test-only minting code with no production path reaching them
(FND-001), recorded as an observed deviation from ADR-011 §6.1's minting rule
rather than an addition to that rule. One already-tracked, differently-owned
violation is left untouched (FND-002), because its correct remediation needs
a public-API change gated behind the #213 S-2 kernel-extraction lane
(ADR-013 T-6, O-05), out of this review's minting-path scope. This is not
owner acceptance.

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
| FND-001 | medium | `value::enumeration` (`enumeration.rs:117,162`, called from `EnumDeclaration::admit`/`admit_member` at `enumeration.rs:181,209`) and `value::unit` (`unit.rs:198,311`, called from `UnitGraph::admit` at `unit.rs:449`; line numbers as of this review's declared baseline, `236bee7`) call `node_key_of`, computing a fresh SHA-256 digest over a JCS preimage and comparing it to a caller-supplied key — the same operation `value::expression::check` performs. Neither module is reached by a check-stage caller today: `grep -rn "EnumDeclaration::admit\|UnitGraph::admit" --include="*.rs" .` at 236bee7 finds every caller is a test — `tests/text_enum_identity.rs`, `tests/equality_matrix.rs`, `tests/quantities.rs`, `tests/collection_algebra.rs`. No `src/` module calls them: `model::checked_dispatch.rs:1019` builds its `PackageDeclarations` with `..PackageDeclarations::default()`, leaving `enums: Vec<EnumBinding>` empty, and no `src/model/intake` module exists to populate them either. Issue #118, which landed both files, scoped its exit criteria to "preserve I04/I05 compatibility... without minting a checked package or claiming runtime execution authority" — deliberately built as a standalone identity/admission primitive, not wired into a check-stage caller. **Verdict: `value::enumeration` and `value::unit` are not S3 implementers today.** They are test-only minting code with no production path reaching them, not an incomplete entry on ADR-011 §1's check-stage "today" list. (This review's first draft claimed otherwise, on two grounds now withdrawn: that "per ADR-011 §6.2, layer 3 as a whole implements S3," and that `value::library` was §1's precedent for a layer-3 module outside `check` still counting as an S3 "today" implementer. Both are wrong. §6.1's layer-3 row's Stage cell reads, verbatim, "S3, I1, I2 binding and view" — layer 3 is a multi-stage layer with named sub-layers, one of which is `check` core; §6.2 maps both `value::enumeration` and `value::unit` to the `semantic_value` sub-layer, not `check`. And `value::library` does not call the `NodeKey` constructor at all, so it cannot serve as precedent for minting outside `check` either way.) ADR-011 §6.1's "Kernel identity constructors have one caller each... [o]nly `check` calls the kernel `NodeKey` constructor" (ADR-013 O-04) and T-12's enforcement of it are correct as written and are not amended for this: a gate that reads that normative statement still — correctly — fails these four call sites, by design, until a check-stage caller exists. **Fix (this branch):** no change to ADR-011 §1. The deviation is recorded as new row FB-13 in §3 "Forbidden bypasses" / "Observed today", not in the §1 ownership column: `value::enumeration`'s `EnumDeclaration::admit`/`admit_member` and `value::unit`'s `UnitGraph::admit` call `node_key_of` today with no check-stage caller and no production reachability. Remaining work: #131 (the family-checker ticket that wires `model::intake`/`check` to call `admit`/`admit_member` for real domain packages). | ADR-011 §3 FB-13, §6.1 layer-3 row and "Kernel identity constructors" bullet, §6.2 `value` non-kernel row; `src/model/checked_dispatch.rs:1019`; #118; #131 |
| FND-002 | medium | `src/value/model_query.rs:108`'s `NodeKey::from_bytes` call reinterprets an `EffectiveId`'s bytes as a `NodeKey` with no digest computed — not a mint in O-04's sense, but a domain-conflating byte transfer ADR-013 O-05 already names and rejects: "the `EffectiveId` ↔ `NodeKey` transfers at `value/model_query.rs:108,123,155` have no canonical role" (ADR-013 §O-05, resolving OBS-018), and O-04's own "Public type" field already decides `NodeKey::from_bytes` (`value/node.rs:49`) is removed. `value::model_query` is ADR-011 §6.2's layer-3 `model` row, not `check`, so this is a real O-04 violation, already correctly documented — nothing in the spec text needed correcting here. The only sound fix is the one ADR-013 already names: retype `ObjectReference::object_type` (`src/value/reference.rs:65`) as `EffectiveId` instead of `NodeKey`, eliminating the transfer. That field is read by `TypeEnvironment::object_type` (`src/value/reference.rs:139,168`) and by `src/value/composite.rs` and `src/value/expression/evaluate.rs`, so retyping it is a crate-wide, public-API-changing move of the kind ADR-013 T-6 assigns to **#213 S-2** (ADR-013 O-05's "Implementing ticket" row, not S-1) — the serial lane #211's own task brief places out of scope ("keep the change to the minting path and do not refactor around it"). **Left untouched on this branch, per owner ruling.** T12-B's actual pattern list, verified at `origin/task/215-integration-lanes@daa9b35`, is `["NodeKey::of(", "NodeKey::from_bytes(", "node_key_of("]`, and `value::model_query` is not an allowed caller prefix — so T12-B does, correctly, still flag `model_query.rs:108` today; this finding does not, and was never going to, make T12-B pass. (This review's first draft claimed T12-B "no longer examines `NodeKey::from_bytes` calls at all" after an assumed pattern change scoped to #211; that claim was never verified against #249's actual branch content and is withdrawn.) The escalation is unchanged: this remains T12-B/O-05's open item, separately tracked under #213 S-2. | ADR-013 O-04 Public type field, O-05 Conversions/closing note (OBS-018) and Implementing ticket row, T-6; `src/value/reference.rs:63-87,139,168`; `origin/task/215-integration-lanes@daa9b35` |

## Disposition

- FND-001: resolved on this branch by recording the deviation, not by
  widening the rule. `spec/decisions/ADR-011-stage-dag-and-dependency-
  architecture.md` §1's S3 "today" list is unchanged (still the original 3
  modules); a new row, FB-13, is added to §3 "Forbidden bypasses" /
  "Observed today" naming `value::enumeration` and `value::unit` as a
  deviation from §6.1's minting rule, with no relaxation of ADR-013 O-04,
  §6.1, or T-12. Cross-reference doc comments were rewritten (not merely
  added) at `src/value/enumeration.rs:11-19` and `src/value/unit.rs:11-19` to
  match: they no longer claim S3-"today"-implementer status or a
  `value::library` analogy, and now cite FB-13 and #131. No code behavior
  changed: `EnumDeclaration::admit`, `admit_member` and `UnitGraph::admit`
  are unchanged, so no new test is required for this finding.
- FND-002: escalated, not decided here. Options for the ADR-013/#213 owner:
  (a) leave as-is, tracked under #213 S-2, and let T12-B continue to
  correctly fail there until #213 lands; (b) pull the `ObjectReference`
  retyping forward as its own ticket ahead of the rest of #213, if the owner
  wants T12-B green sooner. This review does not choose between them.
- M-1, what #249 (`task/215-integration-lanes`) owes for T12-B once this PR
  lands: **nothing to #249's rule or allow-list.** #249's T12-B pattern list
  (`["NodeKey::of(", "NodeKey::from_bytes(", "node_key_of("]`, no
  `value::enumeration`/`value::unit`/`value::model_query` caller exemption)
  is unchanged by this PR, and per R8/FB-13 it should stay unchanged: T12-B
  is supposed to keep failing on `enumeration.rs:117,162`, `unit.rs:198,311`
  and `model_query.rs:108` until #131 (enum/unit) and #213 S-2 (model_query)
  land — that is the correct, by-design state FB-13 and FND-002 both
  document, not a defect in #249's rule. What #249 does still owe, once this
  PR merges: its own review/PR text should stop describing these four
  call sites as an unexplained or open-ended T12-B red, and instead cite
  ADR-011 FB-13 (#131) for the enumeration/unit pair and ADR-013 O-05 /
  #213 S-2 for `model_query.rs:108` as the tracked, expected cause — a
  documentation update inside #249, not a code or gate-rule change. This
  review does not edit #249's branch; it reports this to the coordinator for
  #249 to action.
