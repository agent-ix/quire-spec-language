---
id: FR-141
title: "Evaluate text and declaration-qualified enumerations"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

# FR-141: Evaluate text and declaration-qualified enumerations

## Description

When checking text or enumeration values, the semantic kernel SHALL apply the
selected text normalization and declaration-qualified member identity rules.

## Inputs

Text profile, text bounds, enum declarations, literals and comparison requests.

## Outputs

Completed typed text/enum values or comparison results, or an undefined,
refused or incomplete evaluator outcome. I13 dispositions are not evaluator
outputs.

## Behavior

Text equality and ordering operate on the selected Unicode scalar/normalization
profile and enforce declared length bounds. Enumeration members are identified
by declaration identity plus member identity; source order defines ordering
only when the declaration explicitly selects ordered enumeration semantics.
Evolution never reuses a removed member identity for different meaning.

## Text and enumeration profiles

`unicode-scalars` compares the decoded scalar sequence without normalization;
`nfc`, `nfd`, `nfkc` and `nfkd` apply exactly the named Unicode normalization
form using the `quire.value.text.unicode-17.0.0/v1` tables; `binary-utf8`
compares the admitted UTF-8 payload bytes. A source string's payload is the
canonical UTF-8 encoding of its decoded scalar sequence, so raw `é` and
`\u00e9` have one binary value while their different source spellings remain
provenance. A runtime text input supplies its validated payload bytes directly.
Length is scalar count for the first five profiles
and byte count for `binary-utf8`. Ordering is lexicographic over normalized
scalar values or unsigned bytes respectively. No locale, case folding,
collation table or grapheme segmentation is implicit.

An enumeration value is (`declaration identity`, `member identity`). Unordered
enumerations admit only equality/inequality. An `ordered enum` additionally
uses declaration order for comparison; insertion or reordering therefore
creates a new declaration revision. Optional display strings never participate
in equality or ordering.

Declaration identity is the opaque node key of an I04 `scalar_type`/`enum`
node. Its `quire.checked-semantic-node/v1` digest is SHA-256 over RFC 8785 JCS of
`{ version: quire.enum-declaration-node/v1, owner, qualified_declaration,
ordered, members }`, where `owner` is the stable subject projection of an exact
admitted source, DefinitionRef or ModelRef (and must join that lock selection)
and `members` is the declaration-ordered list of semantic case names
(sorted by case name for an unordered enum). Member identity is the opaque key
of an I04 `value`/`enum_value` node whose `semantic_type` references that
declaration; its digest preimage is
`{ version: quire.enum-member-node/v1, declaration_node_id, case }`. Display
text and source loci are excluded from both preimages. The enum-value node has
that declaration as its sole dependency and its enum literal body carries the
same case; the strict reader joins all three fields to the preimage.

The strict I04 reader recomputes those enum node digests from the admitted
semantic graph. Retaining a key while changing owner, declaration membership,
case meaning or ordered position is `invalid_semantic_graph`. Recomputing after
such a change yields a new declaration/member identity. The same exact imported
owner and declaration retain the same keys across otherwise different packages;
an unrelated package edit cannot rename them. Neither key is a local name,
source position or caller-computed assertion. Under
`quire.value.accounting/v1`, text normalization charges the named input,
normalization and result points before publishing a value; exhaustion is
incomplete, not a truncated string.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-141-AC-1 | Canonically equivalent text compares according to the selected normalization profile and distinct text remains distinct. | Test (TC-186) |
| FR-141-AC-2 | Equal member spellings from different enum declarations are not equal. | Test (TC-186) |
| FR-141-AC-3 | Missing text bounds, invalid scalars, unordered enum ordering or an enum node whose content does not match its normative key preimage refuses. | Test (TC-186) |
| FR-141-AC-4 | Each text profile applies its exact length and lexicographic comparison domain; equal display text under different profiles remains differently typed. | Test (TC-186) |
| FR-141-AC-5 | An unordered enumeration comparison other than equality refuses, while an ordered enumeration follows declaration order only. | Test (TC-186) |
| FR-141-AC-6 | Exact-bound accounting succeeds and denial of a named next text or enum charge returns incomplete without a truncated value or Boolean. | Test (TC-186) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
