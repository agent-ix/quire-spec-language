<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13741 method=spec-review/spec-integrity-analysis lang=spec pr=quire-spec-language#1175 model=claude-opus-5-5 run=29FAD705-13ED-41F1-8BE5-6D0653015F44 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | other | FR-064 sets SHALL obligations on a crate QSL does not own |
| FND-002 | low | other | ADR-032 R-3 and its rejected alternative 'A `#[canonical]` attribute from `qsl-attrs`' still treat `qsl-attrs` as QSL's marker-attribute crate |

SR file: reviews/qsl-951-pr1175-spec-review-integrity.md (SR-13741). Verdict: Mergeable on this lens once FND-001 is resolved: QSL obligations should be stated as dependencies on ix-trace-rs, not as SHALLs on it. FND-002 is low.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: "FR-064 owner statement", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "choose behaviour. The attribute `#[string_edge]` marks an edge function, and it is owned by the neutral crate `ix-trace-rs` (agent-ix/ix-trace-rs), which provides it beside `#[trace]` and `#[implements]`. QSL SHALL provide the scan that reports every string comparison or string `match` outside a marked function in the selected crates' non-test code, so this rule is enforced by a running check rather than by convention alone. QSL SHALL define no `string_edge` attribute of its own."}
  - {id: "FR-064 marker no argument", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL take no argument; a call with any argument SHALL be a compile error that retains the annotated item, so the diagnostic is not followed by a stack of errors from the missing function."}
  - {id: "FR-064 marker identity", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL expand to the annotated item unchanged and carry no runtime behaviour: its presence or absence SHALL NOT change what the marked function compiles to."}
  - {id: "FR-064 marker crate deps", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "`ix-trace-rs` SHALL have no normal dependency, no dependency on QSL, and no dependency on a model, IR or code-generation crate, so a consumer depends on it directly."}
  - {id: "FR-064 import rule", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "QSL and every consumer SHALL import the attribute from `ix-trace-rs` (`use ix_trace_rs::string_edge;`) or write its qualified form `#[ix_trace_rs::string_edge]`. The QSL workspace SHALL hold no definition, re-export or alias of `string_edge`, and SHALL hold no crate whose only purpose is that marker; the scan recognises the attribute by its final path segment `string_edge` and by that name only."}
  - {id: "FR-064 consumer marker", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "The marker is `#[string_edge]`, including its qualified form `#[ix_trace_rs::string_edge]`; consumers that compile marked functions SHALL obtain the `ix-trace-rs` crate by Cargo dependency on its repository (agent-ix/ix-trace-rs). A marker identifies the one total conversion at a permitted edge; it is not a consumer-defined exemption for an entire crate or file."}
  - {id: "FR-064 Dependencies", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-codegen#86 run the same scan in their own repositories, over the same rule; this requirement builds the attribute and the scan tool that #141 and #86 invoke, not their own edge inventories."}
  - {id: "FR-064-AC-13", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-13 | A consumer crate that applies `#[string_edge]` from `ix-trace-rs` to a function compiles and runs the function with the behaviour of the unmarked function; `#[string_edge(anything)]` fails to compile with the one error \"`#[string_edge]` takes no arguments\" and no further error from the retained function. | Test (TC-162)"}
  - {id: "FR-064-AC-15", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-15 | `cargo metadata` over the QSL workspace lists `ix-trace-rs` as the provider of `string_edge`, and `ix-trace-rs`'s own manifest lists no normal dependency and no QSL, model, IR or code-generation package; the layer table of ADR-011 and ADR-012 §9 names `ix-trace-rs` as the exempt marker crate. | Test (TC-162)"}
  - {id: "ADR-012 section 9 marker", path: "spec/decisions/ADR-012-semantic-family-extension-contracts.md", role: examined, excerpt: "edge function carries the marker attribute `#[string_edge]`, a tool attribute owned by the neutral crate `ix-trace-rs` (agent-ix/ix-trace-rs) and imported from it; QSL defines no marker of its own. An `xtask string-edge` scan over the QSL crates reports every string comparison or string `match` outside a marked function in non-test code, and runs in the lint gate. Each allow-list entry names the user value it compares and selects no behaviour. #214 builds the scan; the attribute is `ix-trace-rs`'s."}
  - {id: "ADR-012 work table #214", path: "spec/decisions/ADR-012-semantic-family-extension-contracts.md", role: examined, excerpt: "`FamilyKind`, `FamilyContract`, `ReferenceEvaluation`, seams S1–S4, the seam probe, the thin check seam for function application, the `xtask string-edge` scan (its `#[string_edge]` attribute is `ix-trace-rs`'s) | #214"}
  - {id: "ADR-011 X-10", path: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md", role: examined, excerpt: "X-10 | Extract crate `qsl-replay`. **Extracted**: `bounds`, `call_site`, `execute`, `identity`, `proof_result`, `request`, `result`, `spine` and `witness` are in the crate `qsl-replay`. Its `[dependencies]` are exactly `qsl-cst` (1), `qsl-forms` (2), `qsl-semantics` (3), `qsl-package` (4), `qsl-eval` (5), `qsl-foundation` (F), `quire-semantic-value` (SV: the `InputRefusal` and `Location` the spine reports), `quire-exact` (K), the marker crate `ix-trace-rs` (ADR-012 section 9) and `thiserror`, and `qsl-source` (I3) under its feature `quire-extraction`; the facade re-exports items of `qsl-semant"}
  - {id: "ADR-011 crate map", path: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md", role: examined, excerpt: "Crate map. Layers F, 1, I3, 2, 3, 4, 5, R, A, P, X and the layer-6 `replay` facade are each their own workspace crate (§7.2). K is `quire-exact` (X-1), SV is `quire-semantic-value` (X-11), W is `quire-walk`, an external crate in its own repository, `agent-ix/quire-walk`, and WG is `qsl-walk-grow` (ADR-030 FR-356). A layer crate's `[dependencies]` name only the layer crates and the external crates in its \"Depends on\" cell above, plus external third-party crates that §6.1 says are not layer"}
  - {id: "ADR-032 R-3", path: "spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md", role: examined, excerpt: "R-3 | A marker attribute from `qsl-attrs` would need a new dependency in `quire-exact` and `quire-semantic-value` and an FB-05 change. | No new dependency and no reserved-name list, since a list is a ledger. Canonical types carry the doc-comment tag `/// quire:canonical`, which the xtask parses (DT-1). FB-05 is unchanged."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: medium
    method: spec-review/spec-integrity-analysis
    check: other
    artifact_id: "FR-064 marker crate deps"
    related_ids: ["FR-064 marker no argument", "FR-064 marker identity", "FR-064-AC-13", "FR-064-AC-15"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "72-74"
    excerpt: "`ix-trace-rs` SHALL have no normal dependency, no dependency on QSL, and no dependency on a model, IR or code-generation crate, so a consumer depends on it directly."
    finding: "FR-064 sets SHALL obligations on a crate QSL does not own. Examples: '`ix-trace-rs` SHALL have no normal dependency, no dependency on QSL ...', and the marker 'SHALL take no argument; a call with any argument SHALL be a compile error that retains the annotated item'. AC-13 also fixes ix-trace-rs's diagnostic text. No QSL change can meet or break these. A QSL coder cannot implement them, and ix-trace-rs's own spec/, which is where they belong, does not hold them. Recast them as the contract QSL depends on, for example 'QSL relies on ix-trace-rs providing ...' under Dependencies, and have the ix-trace-rs ticket from SR-13740 FND-001 own them as ix-trace-rs requirements. AC-13 and AC-15 then remain QSL-side conformance checks of that contract."
  - fnd: FND-002
    severity: low
    confidence: low
    method: spec-review/spec-integrity-analysis
    check: other
    artifact_id: "ADR-032 R-3"
    related_ids: []
    path: "spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md"
    lines: "141"
    excerpt: "R-3 | A marker attribute from `qsl-attrs` would need a new dependency in `quire-exact` and `quire-semantic-value` and an FB-05 change. | No new dependency and no reserved-name list, since a list is a ledger. Canonical types carry the doc-comment tag `/// quire:canonical`, which the xtask parses (DT-1). FB-05 is unchanged."
    finding: "ADR-032 R-3 and its rejected alternative 'A `#[canonical]` attribute from `qsl-attrs`' still treat `qsl-attrs` as QSL's marker-attribute crate. Once this PR's AC-14 holds, no `qsl-attrs` crate exists. The reasoning still holds for ix-trace-rs, since a marker crate would still add a dependency to K and SV. A reader of the decision record, though, will look for a crate that is gone. Name the marker crate generically ('a marker attribute crate, now ix-trace-rs'). Or leave the text as a dated rejected alternative if decision records are meant to keep their wording."
bindings: []
```

+++
