---
id: FR-014
title: "Bind exact native source to formal coordinates"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: depends_on
---
# FR-014: Bind exact native source to formal coordinates

## Description

When a caller maps a source span through an explicit native-to-formal source binding, the source bridge shall return coordinates for the exact bound source.

## Inputs

An immutable native Source and a caller-selected, constructor-validated Contract
IR SourceIdentity establish one FormalSource binding. Forward requests supply
the requesting Source and a half-open native Span. Reverse requests supply a
constructor-validated IR SourceSpan. The Rust API and trust boundary are defined
in [the bridge contract](../../docs/formal-source-binding.md).

## Outputs

A formal SourceSpan or a native Span respectively. A rejected request returns
Box<Diagnostic> with invalid_source_map and source_map phase, located at byte
zero of the bound native source. The binding exposes both identities and the
original immutable Source, including its exact byte digest.

## Behavior

The source bridge shall retain the supplied native and formal identities without deriving either identity or revision from the other.

If a forward request differs in native identity, revision, display path or byte digest, then the source bridge shall reject the request.

When a valid forward span is mapped, the source bridge shall derive both formal endpoints from the original UTF-8 bytes using one-based lines and Unicode scalar columns.

If a reverse span has a foreign formal identity or endpoints inconsistent with the bound bytes, then the source bridge shall reject the request.

The source bridge shall preserve its binding across successful and failed requests.

Both directions admit empty spans, EOF and CRLF-interior boundaries; LF alone
advances the line. Neither direction admits reversed, out-of-range or
split-scalar spans. Reverse mapping compares every line, column and byte offset;
a constructor-valid IR span is not sufficient evidence of source correspondence.
All integer conversions are checked. An unexpected IR constructor refusal is
retained in Diagnostic.upstream; ordinary local refusals have no upstream or
related declarations.

Binding assigns correspondence at the caller's authority; it does not establish
global identity uniqueness, an authored requirement owner, semantic typing or
source authenticity. Source::read_verified remains the existing pinned-byte
intake. Extracted-to-original mapping remains the separate SourceMap API.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-014-AC-1 | An opaque native revision such as draft:alpha and a separately supplied formal revision 91 remain distinct and inspectable with the exact original Source and digest. | Test (TC-035) |
| FR-014-AC-2 | Forward mapping reports independently expected byte, line and scalar-column endpoints for ASCII, CRLF, multibyte scalars, empty source, empty spans, EOF and the existing 1 MiB source boundary. | Test (TC-036) |
| FR-014-AC-3 | Changing only a request's native identity, revision, display path or bytes refuses with invalid_source_map; a subsequent exact-source request still succeeds. | Test (TC-037) |
| FR-014-AC-4 | Reverse mapping rejects a foreign formal document/revision, inconsistent line/column, split scalar, out-of-range offset or u64::MAX offset without manufacturing a native locus. | Test (TC-038) |
| FR-014-AC-5 | For every generated source in TC-039's finite family, valid spans agree with an independent coordinate oracle and round-trip; invalid spans refuse and request order does not change results. | Test (TC-039) |

## Dependencies

- [US-002](../usecase/US-002-link-exact-models.md) requires exact source/model correspondence.
- [FR-001](FR-001-read-exact-source.md) owns immutable intake and its caller-lowered 1 MiB ceiling.
- [FR-010](FR-010-report-native-outcomes.md) owns located native diagnostics.
- [NFR-005](../non-functional/NFR-005-rust-verification-paths.md) requires Rust production and qualification paths.
- [FR-006](FR-006-check-defined-expressions.md) remains the downstream checking obligation; this bridge does not satisfy TC-025–029.

## Status

Implemented and qualified source correspondence for native checking. No checker or execution completion is claimed.
