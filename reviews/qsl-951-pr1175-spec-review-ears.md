---
id: SR-13742
title: 'Spec review of PR #1175: ix-trace-rs owns the string_edge marker, EARS conformance'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@76a302455; git diff origin/main...76a302455 (PR #1175, branch qsl-951-trace-marker): ADR-011 X-10, ADR-012 section 9 and work table, FR-064, TC-162; read against agent-ix/ix-trace-rs main (Cargo.toml, README.md, src/lib.rs, tags v0.1.0/v0.1.1), the QSL workspace manifests and qsl-string-edge/src/detector.rs at origin/main, IR FR-028 on quire-contract-ir main, and the proposal in IR-212 comment 855ebf10 and IR-9 comment a884990a; ticket QSL-951'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-011
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-012
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-064
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-162
  type: reviews
---

## Summary

Ticket: QSL-951. PR quire-spec-language#1175, head 76a302455. EARS lens over every behaviour statement the diff adds or rewrites in FR-064. ADR-011, ADR-012 and TC-162 are decision or procedure prose, not requirement statements.

`make check-spec-validation` reports no `ears:` or `ac:` diagnostic on FR-064 at the PR head or the merge base. The new SHALL statements use the ubiquitous pattern. 'QSL SHALL provide the scan ...' and 'QSL SHALL define no `string_edge` attribute of its own' are well formed. The findings below are about bundling, intent stated under SHALL, and subjects outside the system, not grammar the validator checks.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-064 owner statement | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker applicable | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker no argument | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker identity | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker denotes | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker crate deps | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 import rule | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 consumer marker | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Two statements hold more than one SHALL. The no-argument bullet is 'It SHALL take no argument; a call with any argument SHALL be a compile error that retains the annotated item'. The import paragraph has 'QSL and every consumer SHALL import ...', then 'The QSL workspace SHALL hold no definition ... and SHALL hold no crate ...', then a descriptive clause about the scan. Split each into one SHALL per statement, and make the scan's recognition rule its own 'The scan SHALL recognise ...' statement. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:64-66; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:76-81 |
| FND-002 | low | 'It SHALL denote the one total conversion at a permitted edge, and it is no exemption of a file or a crate' puts an intent ('denote') under SHALL. No test can observe it, and the sentence joins it to a descriptive clause. The testable part is placement: the scan treats the marker as covering only the marked function, which the allow-list rules and FR-064-AC-8 already state. Rewrite it as a non-normative note, or as 'The scan SHALL treat a marker as covering only the function it is applied to'. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:70-71 |
| FND-003 | low | Two subjects sit outside the system. 'QSL and every consumer SHALL import ...' binds every consumer, which QSL cannot verify. '`ix-trace-rs` SHALL have no normal dependency ...' is about another crate (see SR-13741 FND-001). Write the QSL statement with QSL as its subject ('The QSL workspace SHALL import ...'), and state the consumer rule as guidance, or as a consumer documentation requirement. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:76-81; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:72-74 |

## Verdict

Mergeable on this lens alone. Three low findings. None changes behaviour.

## Dispositions

Round 1, head df77f17c3 (origin/main merged at f47731b39). `make check-spec-validation` and `tools/check-index-completeness.sh` exit 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in df77f17c3: - The scan SHALL treat a function as marked when it carries an attribute whose path is `string_edge` or `ix_trace_rs::string_edge`. - The scan SHALL NOT treat any other path, a crate-qualified path of another crate included, as a marker. - The scan SHALL treat a marker as covering only the function it is applied to, never a file or a crate. - The QSL workspace SHALL import the attribute from `ix-trace-rs` and declare that dependency in the manifest of each package that uses it. - The QSL workspace SHALL define no item named `string_edge`: no proc-macro attribute, no re-export and no alias. |
| FND-002 | fixed | Fixed in df77f17c3: - The scan SHALL treat a marker as covering only the function it is applied to, never a file or a crate. |
| FND-003 | fixed | Fixed in df77f17c3: - The QSL workspace SHALL import the attribute from `ix-trace-rs` and declare that dependency in the manifest of each package that uses it. - The QSL workspace SHALL define no item named `string_edge`: no proc-macro attribute, no re-export and no alias.  A consumer imports the attribute from `ix-trace-rs` (`use ix_trace_rs::string_edge;`) or writes `#[ix_trace_rs::string_edge]`; a consumer that keeps its own copy of the marker is outside what the scan can detect by path. |
