---
id: SR-20261109172912000126
title: "QSL-1813 source diagnostic wire gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1813; qsl-replay/src/outcome.rs; qsl-replay/src/spine.rs; qsl-replay/tests/it/main.rs; qsl-replay/tests/it/source_diagnostic_outcome.rs; FR-286-AC-15; TC-770 step 12"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
---

## Summary

Independent gap-analysis of the producer-owned source diagnostic projection and its real external integration tests. No defects found in the scoped delta.

## Verdict

**PASS for the manual scoped semantic delta check only; whole-repository computed-matrix assurance was not completed and is not claimed.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Evidence

Frozen head: `d458ee16ad8d847d11af296f67cc31acc51c0ca5`. Ticket: QSL-1813. Changed files are qsl-replay/src/outcome.rs, qsl-replay/src/spine.rs, qsl-replay/tests/it/main.rs and qsl-replay/tests/it/source_diagnostic_outcome.rs. No dependency manifest, CI lane, wire reader, source validator or core I/O boundary changed.

The public projection borrows the real CompleteDiagnostic, obtains its cause from CompleteCause::as_str and label only from the owner’s typed BlankLabel payload, preserves the optional region through existing OutcomeLocus conversion, and copies the original message. The optional SourceLabel field is private, omitted otherwise, and serialized with SourceLabel::as_str. There is no arbitrary label setter, local JSON insertion, diagnostic-text parsing or fabricated source/locus. The formatter adapter and Source compile-refusal arm delegate to the same projector. CompileRefusal::cause now retains the existing source cause rather than discarding it; the mapping itself remains owner-defined. Existing non-source causes remain unchanged.

External integration tests provide independent literal code/cause/label oracles for all seven SourceReadCause values, distinctive message and region retention, both blank labels and authority precedence, Unicode whitespace, zero-width-space nonblank control, null locus and absent label assertions. Their compile/parse/format comparisons supplement these literal oracles, and actual read/parse cases establish the production route. Removing label retention, dropping the source cause, manufacturing a locus or adding labels to other causes would fail these assertions. ReferenceInvariant is covered by a real publicly constructed refusal because the source owner documents it unreachable from input today.

Independent confined checks passed under flock /tmp/titan-cargo.lock with nice/ionice, Rust 1.98.1, two jobs and two test threads: package formatting; all-target/all-feature Clippy with warnings denied; three new source_diagnostic_outcome tests; eleven existing parse/format/outcome_facade tests. Logs are /tmp/qsl1813-wire-code-review/gate-{fmt,clippy,focused,facades}.log. The aggregate package/consumer/doc gate remains with the code lead after artifact custody. Cargo-deny was not repeated for this dependency-unchanged projection; no dependency or license surface changed. No unsafe, async, global state, narrowing cast, filesystem access, stub or qualification bypass is introduced.

No applicable AssuranceProfile was found under spec/; textual references in historical review documents are not profiles. No assurance exception was applied.

## Coverage

Plan completion: not assessed

Manual scoped trace-tag, reverse-ownership and semantic review only, explicitly authorized by the dispatch. FR-286-AC-15 and TC-770 step 12/its expected result are examined; other TC-770 steps are context, not newly qualified. Three real non-ignored tests carry TC-770/FR-286-AC-15. No scoped unowned behavior, stub, vacuous guard or inflated full-criterion claim was found.

Whole-repository computed Test Matrix and runtime coverage were NOT COMPLETED and are not claimed: the dispatch prohibits repeating earlier 521-second/295-second probes that returned no JSON. This is not a repository-wide gap-analysis PASS and does not complete QSL-1813, driver #141, or the command-intake work.

## Examined Units

```yaml
{
  "scope": [
    {
      "id": "FR-286-AC-15",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "An external caller projects a pre-admission blank-label source diagnostic through `from_source_diagnostic` and serializes a document: its code is `invalid_source_identity`, cause `blank-label`, `label` the exact typed `authority` or `identity`, `locus` null and message unchanged. Empty-path, byte-budget and located invalid-UTF-8 source diagnostics retain their exact cause, code, optional region and message while omitting `label`; the located case retains its original source reference and endpoints. Projecting the same source diagnostic through a compile refusal, a parse refusal or a format"
    },
    {
      "id": "TC-770",
      "path": "spec/test-cases/TC-770-library-outcomes-serialize-to-one-outcome-document-with-the-undefined-label-on-non-proof-outcomes-only.md",
      "role": "examined",
      "excerpt": "12. From outside the `qsl-replay` crate, construct FR-001's typed pre-admission blank-label refusals for `authority` and `identity`, and project their complete diagnostics with `OutcomeDiagnostic::from_source_diagnostic`. Serialize a document containing each diagnostic. Repeat for an empty-path refusal, a byte-budget refusal with no region and an invalid-UTF-8 refusal with its original region. Compare each diagnostic with the same source refusal through compile, parse and format outcome producers, using actual source-operation refusals where reachable."
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
      "test_id": "pre_admission_blank_labels_retain_the_typed_label_without_a_locus",
      "ac_id": "FR-286-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "other_source_causes_omit_label_and_preserve_the_original_region_and_detail",
      "ac_id": "FR-286-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "actual_source_reads_and_parse_refusals_use_the_same_projection_bytes",
      "ac_id": "FR-286-AC-15",
      "trace": "correct"
    }
  ]
}
```
