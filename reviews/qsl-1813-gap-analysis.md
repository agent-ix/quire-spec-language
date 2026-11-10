---
id: SR-20261109172912000107
title: "QSL-1813 CST source refusal gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1813; qsl-cst/src/diagnostic.rs; qsl-cst/tests/it/main.rs; qsl-cst/tests/it/source_refusal.rs; FR-001-AC-13..15; TC-424"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: references
---

## Summary

Independent gap-analysis of the public source-refusal projection and external integration tests. No defects found in the scoped delta.

## Verdict

**PASS for the manual scoped semantic delta check only. Whole-repository computed-matrix assurance was not completed and is not claimed.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Evidence

Frozen head: `f5bf7eacd7bd2db3a4db507bcab23322ef067cad`. Ticket: QSL-1813. Only qsl-cst/src/diagnostic.rs, qsl-cst/tests/it/main.rs and qsl-cst/tests/it/source_refusal.rs changed. No manifest, wire shape or CI workflow changed. FR-001 projection spec is already merged through PR #2632.

The public mapper consumes the existing owned refusal and exhaustively maps its seven causes. Original source, display path, message and optional region move unchanged; no reference is minted and no input is read. Only ByteBudget adds the supplied SourceBytes bound. read_source delegates to this single mapper. The intended production consumer is driver label refusal before file I/O; this is not a test-only API.

The integration test's seven explicit expected code/cause pairs independently detect mapping errors. Its distinctive labels/path/message and original located regions detect metadata loss; absent regions, Source phase and empty related regions are asserted. Ceilings 4 and 2,097,152 cover both sides of the default. Blank-label cases exercise each label, both blank, empty/space/U+3000 and U+200B, with identity_fields assertions. The supplementary read_source comparison checks six reachable refusals; ReferenceInvariant is correctly covered through public construction because foundation documents it unreachable from input today.

Independent confined gates passed: cargo fmt -p qsl-cst --check; cargo clippy -p qsl-cst --all-targets --all-features -- -D warnings; cargo test -p qsl-cst --test it source_refusal:: (3 passed). Thermal flock, nice/ionice, two jobs and two test threads applied. Log: /tmp/qsl1813-projection-review-gates.log. No full crate/consumer/docs aggregate or cargo-deny was run by this reviewer; parent owns final affected aggregate. Dependencies are unchanged. No unsafe, async, filesystem, parsing, narrowing cast, stub, duplicate mapper or copied cross-repository material was introduced. No applicable AssuranceProfile was found under spec/ (textual mentions in older review documents are not profiles).

## Coverage

Plan completion: not assessed

Reconciliation: manual scoped trace-tag and semantic inspection, not a computed Test Matrix. Earlier whole-repository matrix/coverage probes returned no JSON after 521/295 seconds and were stopped; the dispatch prohibits repeating these unbounded probes. No whole-repository traceability, runtime coverage or matrix PASS is claimed. All three changed criteria have real, non-ignored tagged tests (four test-to-criterion bindings). Zero scoped unowned behaviors or stubs found. Semantic review ran over FR-001-AC-13..15 with explicit authorization in the review brief. TC-424 examination is confined to steps 8–10 and their expected results; its older steps are context, not newly qualified here.

This PR does not complete FR-287, the driver canonical CLI port, wire label propagation, or QSL-1813. Driver #141 remains unqualified.

## Examined Units

```yaml
{
  "scope": [
    {
      "id": "FR-001-AC-13",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "A caller outside CST constructs a typed blank-label refusal using `first_blank_label` and the existing public refusal fields, then calls `source_refusal` without bytes or an admitted source. For empty, space-only and U+3000 labels, including both labels blank, the projected identity cause retains the first blank label in authority-then-identity order; U+200B is not blank. An empty-path refusal retains `empty-path` with no label."
    },
    {
      "id": "FR-001-AC-14",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "For every `SourceReadCause`, public projection gives the code and complete cause in the mapping above, retains source identity, path, message and optional region exactly, sets phase `Source` and has no related regions. Cases with no region retain `None`; located UTF-8, BOM and NUL cases retain their original reference and endpoints."
    },
    {
      "id": "FR-001-AC-15",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "Projecting `ByteBudget` under ceilings below and above 1 MiB retains exactly the supplied bound as `SyntaxLimit::SourceBytes`; projecting every other cause under either ceiling has no syntax limit. Source refusals returned through `read_source` have the same projected fields as direct public projection of the corresponding `Source::read_typed` refusal with the same ceiling."
    },
    {
      "id": "TC-424",
      "path": "spec/test-cases/TC-424-a-read-source-carries-its-caller-named-source-reference.md",
      "role": "examined",
      "excerpt": "Scope: FR-001-AC-5 to FR-001-AC-15."
    },
    {
      "id": "FR-001",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "context_only",
      "excerpt": "When source is admitted, the compiler shall retain its exact bytes and the\nsource reference that names them."
    }
  ],
  "bindings": [
    {
      "test_id": "preflight_identity_refusals_need_no_bytes_or_admitted_source",
      "ac_id": "FR-001-AC-13",
      "trace": "correct"
    },
    {
      "test_id": "projection_preserves_each_cause_and_its_original_context",
      "ac_id": "FR-001-AC-14",
      "trace": "correct"
    },
    {
      "test_id": "projection_preserves_each_cause_and_its_original_context",
      "ac_id": "FR-001-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "read_source_uses_the_public_projection_for_actual_refusals",
      "ac_id": "FR-001-AC-15",
      "trace": "correct"
    }
  ]
}
```
