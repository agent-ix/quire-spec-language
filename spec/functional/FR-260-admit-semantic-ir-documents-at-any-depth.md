---
id: FR-260
title: "Admit semantic-IR package documents at any depth under the intake byte limit"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
---
# FR-260: Admit semantic-IR package documents at any depth under the intake byte limit

## Description

Semantic-IR intake (I1, FR-056) SHALL admit or refuse a package document on
its content, whatever its JSON nesting, bounded only by the intake byte
limit (ADR-030 D-4.6).

## Behavior

1. **One read.** Intake SHALL read the document once, through
   `quire-canonical`'s shared reader (FR-259), under `intake.input_bytes`.
2. **Arena tree.** Intake SHALL hold the reader's arena tree as the
   document's derived view (`PackageDocument::tree`) and pass that same tree
   to the semantic-IR crate's checks, so the document is parsed once.
3. **Digest.** The document's `sha256-jcs` digest SHALL encode that tree
   through the event API.
4. **Malformed input.** When the reader refuses the document, intake's
   read (`PackageDocument::parse`) SHALL refuse it
   `invalid_model_binding`/`malformed-declaration` at the document root `$`,
   carrying the reader's byte offset. A lone surrogate escape is one such
   refusal. FR-154 admission does not surface that refusal: it digests the
   bytes the reader refuses raw (FR-056), so they refuse
   `stale_dependency`/`content-mismatch` or
   `invalid_model_binding`/`wrong-model-selection` there.
5. **Byte limit.** When the document is longer than `intake.input_bytes`,
   intake SHALL refuse it as `resource_exhausted`/`intake-limit-exceeded`
   naming the input-bytes limit, its bound, the document's length and
   setting `intake.input_bytes` (FR-255). `intake.input_bytes` has a
   published default of 67108864 bytes, has no ceiling, and is set through
   the intake limits' builder.
   When the read or the digest cannot reserve memory, intake SHALL refuse
   the document `resource_exhausted`/`allocation-failed`, carrying the bytes
   requested (FR-259 B6).
6. **Cycles of any length.** When a document's composite types form a cycle,
   intake SHALL report it as a cycle naming its types, whatever the cycle's
   length.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-260-AC-1 | On a thread with a 512 KiB stack, a package document whose declarations are valid and which holds an array nested 100,000 deep at a member the semantic-IR schema does not admit, with `intake.input_bytes` raised to fit, is refused by FR-056's reader-refusal rule: intake ends with no declaration and retains the `agent-ix-semantic-ir` reader's diagnostic for that member, naming its IR node, artifact id and span, with no `resource_exhausted` cause, and no intake outcome names a depth. Every corpus package FR-056's tests admit is admitted, with the same `sha256-jcs` digest it had. | Test (TC-730) |
| FR-260-AC-2 | `PackageDocument::parse` refuses a package document holding the escape `"\udc00"` in a string `invalid_model_binding`/`malformed-declaration` at `$`, carrying that escape's byte offset. | Test (TC-730) |
| FR-260-AC-3 | A package whose composite types form a cycle of 300 types, and one whose composite types form a cycle of 100,000 types with `intake.input_bytes` raised to fit, are each refused with semantic-IR's composite-cycle refusal naming every type on the cycle, on a thread with a 512 KiB stack. | Test (TC-731) |
| FR-260-AC-4 | A document one byte longer than `intake.input_bytes` at bound `B` is refused `resource_exhausted`/`intake-limit-exceeded` naming the input-bytes limit, bound `B`, actual `B + 1` and setting `intake.input_bytes`. With the setting raised to `B + 1` through the intake limits' builder and through FR-255's settings operation given `intake.input_bytes=<B + 1>` (FR-255), which the driver CLI exposes as `--limit` (ADR-029 CB-1), the same document is judged on its content. | Test (TC-732) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.6 and D-8 overlap item O-4.
- [FR-056](FR-056-admit-domain-package-model-declarations.md) defines
  intake.
- [FR-259](FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md)
  defines the shared reader and event API QSL consumes.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) names
  `intake.input_bytes`.

## Overlap

The semantic-IR crate's move onto `quire-canonical`'s reader tree, and
reporting every composite cycle whatever its length, are Filament-owned
requirements (ADR-030 D-8 O-4); Filament designs them. FR-260-AC-3 tests the
outcome through QSL's intake.

## References

- QSpec FR-460, the ecosystem depth rule (Linear STD-143, which supersedes
  STD-125).
- Linear QSL-381.
