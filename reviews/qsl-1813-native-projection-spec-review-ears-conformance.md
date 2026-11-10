---
id: SR-20261109172912000122
title: "QSL-1813 borrowed native projection spec-review/ears-conformance review"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1813; PR #2636; FR-001 and TC-424"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-001, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-424, type: references }
---
## Summary

Independent spec-review/ears-conformance review of the borrowed native source-refusal projection. No in-scope defect found.

## Assurance Context

No AssuranceProfile exists under spec/. Direct base, EARS, integrity and failure-domain lenses cover the changed projection contract. No enforced workflow or human acceptance is claimed.

## Verdict

PASS for specification readiness after artifact custody. Implementation, command extraction, runtime tests and QSL-1813 completion remain unqualified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Analysis

The new event condition names Foundation and a concrete public conversion. The following ubiquitous obligations name the native projection, identity_cause, span and Source::read; each specifies observable projected data or delegation. The absence/unrenderability condition has a concrete native origin response and expressly preserves the canonical region. Full-tree grammar validation reports 28 unchanged advisories, none in either edited document. No new ambiguous trigger, unverifiable response or subject omission was identified.

## Verification and Limits

Frozen-head and clean-worktree checks pass. Full-tree Quire validation exits 0 with 28 unchanged advisory grammar findings; no finding names an edited document. The existing private native conversion and CST conversion were read for API feasibility. No changed Markdown is embedded in a Rust include. No Cargo, runtime test, Kani, full matrix, source write or push was performed. Dependency/security/object/criterion-strength lenses were not separately run: no relationship edge, authorization boundary or domain entity is added; source identity and failure handling are assessed above. Exact model metadata is unavailable in this harness and is marked unknown rather than guessed.

## Reviewed Units

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FR-001",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "When a caller needs both the retained native diagnostic and the complete\ndiagnostic above, Foundation shall expose its existing conversion as\n`qsl_foundation::diagnostic::source_refusal(refusal: &SourceReadRefusal, bytes: &[u8], byte_limit: usize) -> Box<Diagnostic>`.\nThe projection borrows the existing typed refusal and offered bytes. It\nshall neither admit a source nor repeat label, encoding or byte-budget\nvalidation. The caller can project the native diagnostic, then pass the\nsame owned refusal to CST without constructing a second refusal or a\ndummy source."
    },
    {
      "id": "FR-001",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "The native projection shall retain the source identity, path and message,\nset phase `Source`, and use the existing native code mapping: blank-label\nand empty-path to `InvalidSourceIdentity`, reference-invariant to\n`RuntimeInvariant`, byte-budget to `StageLimitExceeded`, invalid UTF-8 to\n`InvalidUtf8`, and BOM or NUL to `InvalidSyntax`. Its `identity_cause()`\nshall retain the exact blank label or empty-path cause and shall be absent\nfor the other causes. Only byte-budget carries\n`SyntaxLimit::SourceBytes { bound: byte_limit }`."
    },
    {
      "id": "FR-001",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "The native span shall render the refusal's original region over the\noffered bytes with the existing offered-byte coordinate renderer. If\nthere is no region or it cannot be rendered, the native span retains the\nexisting zero-width origin fallback: byte 0, line 1, column 1. This\nfallback belongs only to the retained native diagnostic; the borrowed\nrefusal's region is unchanged and CST still preserves `None` as `None`.\n`Source::read` shall use this same borrowed projection for its typed\nrefusals, with no second conversion."
    },
    {
      "id": "FR-001-AC-16",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "A caller outside Foundation borrows each `SourceReadCause` refusal through the public native projection, then consumes that same refusal through CST's projection. The native code, phase, offered identity, path, message and identity cause match the native mapping above; only byte-budget carries precisely the supplied ceiling, below or above 1 MiB. Located invalid-UTF-8, BOM and NUL cases retain their offered-byte coordinates; an absent or unrenderable region uses the unchanged native origin fallback while the original typed region and CST projection remain unchanged. For every refusal reachable"
    },
    {
      "id": "TC-424",
      "path": "spec/test-cases/TC-424-a-read-source-carries-its-caller-named-source-reference.md",
      "role": "examined",
      "excerpt": "11. From outside Foundation, borrow each typed refusal through\n    `qsl_foundation::diagnostic::source_refusal`, using its original offered\n    bytes and ceiling. Repeat with distinctive identity, path and message,\n    with ceilings below and above 1 MiB, and with no region and an\n    unrenderable region. After the native projection, consume the same\n    refusal through CST's projection. For every source-read refusal\n    reachable through `Source::read_typed`, compare the direct borrowed\n    native projection with `Source::read` under equal inputs and ceiling."
    },
    {
      "id": "TC-424",
      "path": "spec/test-cases/TC-424-a-read-source-carries-its-caller-named-source-reference.md",
      "role": "examined",
      "excerpt": "- Step 11: the native diagnostic retains the exact mapping, phase, identity,\n  path, message, identity cause and byte-budget context FR-001 specifies.\n  Located refusals retain their offered-byte coordinates; absent or\n  unrenderable regions retain only the native zero-width origin fallback.\n  The original refusal remains available and unchanged for CST, whose\n  region stays absent or retains its original endpoints. Direct native\n  projection and `Source::read` agree on every native field, with no\n  second admission or dummy source."
    },
    {
      "id": "FR-001-AC-14",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "context_only",
      "excerpt": "For every `SourceReadCause`, public projection gives the code and complete cause in the mapping above, retains source identity, path, message and optional region exactly, sets phase `Source` and has no related regions. Cases with no region retain `None`; located UTF-8, BOM and NUL cases retain their original reference and endpoints."
    }
  ],
  "findings": []
}
```
