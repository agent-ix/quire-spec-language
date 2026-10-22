<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13742 method=spec-review/spec-ears-analysis lang=spec pr=quire-spec-language#1175 model=claude-opus-5-5 run=29FAD705-13ED-41F1-8BE5-6D0653015F44 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | compound | Two statements hold more than one SHALL |
| FND-002 | low | ambiguous | 'It SHALL denote the one total conversion at a permitted edge, and it is no exemption of a file or a crate' puts an intent ('denote') under SHALL |
| FND-003 | low | ambiguous | Two subjects sit outside the system |

SR file: reviews/qsl-951-pr1175-spec-review-ears.md (SR-13742). Verdict: Mergeable on this lens alone. Three low findings. None changes behaviour.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: "FR-064 owner statement", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "choose behaviour. The attribute `#[string_edge]` marks an edge function, and it is owned by the neutral crate `ix-trace-rs` (agent-ix/ix-trace-rs), which provides it beside `#[trace]` and `#[implements]`. QSL SHALL provide the scan that reports every string comparison or string `match` outside a marked function in the selected crates' non-test code, so this rule is enforced by a running check rather than by convention alone. QSL SHALL define no `string_edge` attribute of its own."}
  - {id: "FR-064 marker applicable", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL be applicable to a function."}
  - {id: "FR-064 marker no argument", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL take no argument; a call with any argument SHALL be a compile error that retains the annotated item, so the diagnostic is not followed by a stack of errors from the missing function."}
  - {id: "FR-064 marker identity", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL expand to the annotated item unchanged and carry no runtime behaviour: its presence or absence SHALL NOT change what the marked function compiles to."}
  - {id: "FR-064 marker denotes", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL denote the one total conversion at a permitted edge, and it is no exemption of a file or a crate."}
  - {id: "FR-064 marker crate deps", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "`ix-trace-rs` SHALL have no normal dependency, no dependency on QSL, and no dependency on a model, IR or code-generation crate, so a consumer depends on it directly."}
  - {id: "FR-064 import rule", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "QSL and every consumer SHALL import the attribute from `ix-trace-rs` (`use ix_trace_rs::string_edge;`) or write its qualified form `#[ix_trace_rs::string_edge]`. The QSL workspace SHALL hold no definition, re-export or alias of `string_edge`, and SHALL hold no crate whose only purpose is that marker; the scan recognises the attribute by its final path segment `string_edge` and by that name only."}
  - {id: "FR-064 consumer marker", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "The marker is `#[string_edge]`, including its qualified form `#[ix_trace_rs::string_edge]`; consumers that compile marked functions SHALL obtain the `ix-trace-rs` crate by Cargo dependency on its repository (agent-ix/ix-trace-rs). A marker identifies the one total conversion at a permitted edge; it is not a consumer-defined exemption for an entire crate or file."}
findings:
  - fnd: FND-001
    severity: low
    confidence: high
    method: spec-review/spec-ears-analysis
    check: compound
    artifact_id: "FR-064 marker no argument"
    related_ids: ["FR-064 import rule"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "64-66"
    excerpt: "It SHALL take no argument; a call with any argument SHALL be a compile error that retains the annotated item, so the diagnostic is not followed by a stack of errors from the missing function."
    finding: "Two statements hold more than one SHALL. The no-argument bullet is 'It SHALL take no argument; a call with any argument SHALL be a compile error that retains the annotated item'. The import paragraph has 'QSL and every consumer SHALL import ...', then 'The QSL workspace SHALL hold no definition ... and SHALL hold no crate ...', then a descriptive clause about the scan. Split each into one SHALL per statement, and make the scan's recognition rule its own 'The scan SHALL recognise ...' statement."
  - fnd: FND-002
    severity: low
    confidence: medium
    method: spec-review/spec-ears-analysis
    check: ambiguous
    artifact_id: "FR-064 marker denotes"
    related_ids: []
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "70-71"
    excerpt: "It SHALL denote the one total conversion at a permitted edge, and it is no exemption of a file or a crate."
    finding: "'It SHALL denote the one total conversion at a permitted edge, and it is no exemption of a file or a crate' puts an intent ('denote') under SHALL. No test can observe it, and the sentence joins it to a descriptive clause. The testable part is placement: the scan treats the marker as covering only the marked function, which the allow-list rules and FR-064-AC-8 already state. Rewrite it as a non-normative note, or as 'The scan SHALL treat a marker as covering only the function it is applied to'."
  - fnd: FND-003
    severity: low
    confidence: medium
    method: spec-review/spec-ears-analysis
    check: ambiguous
    artifact_id: "FR-064 import rule"
    related_ids: ["FR-064 marker crate deps"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "76-81"
    excerpt: "QSL and every consumer SHALL import the attribute from `ix-trace-rs` (`use ix_trace_rs::string_edge;`) or write its qualified form `#[ix_trace_rs::string_edge]`. The QSL workspace SHALL hold no definition, re-export or alias of `string_edge`, and SHALL hold no crate whose only purpose is that marker; the scan recognises the attribute by its final path segment `string_edge` and by that name only."
    finding: "Two subjects sit outside the system. 'QSL and every consumer SHALL import ...' binds every consumer, which QSL cannot verify. '`ix-trace-rs` SHALL have no normal dependency ...' is about another crate (see SR-13741 FND-001). Write the QSL statement with QSL as its subject ('The QSL workspace SHALL import ...'), and state the consumer rule as guidance, or as a consumer documentation requirement."
bindings: []
```

+++
