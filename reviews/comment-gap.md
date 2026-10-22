<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13743 method=gap-analysis lang=spec pr=quire-spec-language#1175 model=claude-opus-5-5 run=29FAD705-13ED-41F1-8BE5-6D0653015F44 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | coverage | Step 14's expected result says the argument case 'reports exactly the one error' but does not give the message |
| FND-002 | low | test-intent | Step 15 says 'resolve each marker' but gives no method |
| FND-003 | low | coverage | Step 9 now asks for the qualified `#[ix_trace_rs::string_edge]` form 'and the imported form' through the library and binary |

SR file: reviews/qsl-951-pr1175-gap-analysis.md (SR-13743). Verdict: Mergeable on this lens once the base review's FND-003 (AC-15) is fixed. Three low findings tighten the procedures.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: "FR-064-AC-12", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-12 | A fixture function marked `#[ix_trace_rs::string_edge]` and one marked with `#[string_edge]` imported by `use ix_trace_rs::string_edge;` each hold a string comparison that the library, the command and QSL's adapter do not report; an unmarked function in the same fixture is reported. | Test (TC-162)"}
  - {id: "FR-064-AC-13", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-13 | A consumer crate that applies `#[string_edge]` from `ix-trace-rs` to a function compiles and runs the function with the behaviour of the unmarked function; `#[string_edge(anything)]` fails to compile with the one error \"`#[string_edge]` takes no arguments\" and no further error from the retained function. | Test (TC-162)"}
  - {id: "FR-064-AC-14", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-14 | The QSL workspace holds no `proc_macro_attribute` definition named `string_edge`, no `pub use` or alias of it, no `qsl_attrs` path and no workspace member `qsl-attrs`; every `string_edge` marker in the workspace's production code resolves to `ix_trace_rs::string_edge`. | Test (TC-162)"}
  - {id: "FR-064-AC-15", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-15 | `cargo metadata` over the QSL workspace lists `ix-trace-rs` as the provider of `string_edge`, and `ix-trace-rs`'s own manifest lists no normal dependency and no QSL, model, IR or code-generation package; the layer table of ADR-011 and ADR-012 §9 names `ix-trace-rs` as the exempt marker crate. | Test (TC-162)"}
  - {id: "TC-162 scope", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "source locations. Scope: FR-064-AC-1 through FR-064-AC-15."}
  - {id: "TC-162 step 9", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "9. Exercise the fixtures from steps 1–5 through the public library and binary with consumer rules. Add qualified `#[ix_trace_rs::string_edge]` coverage and the imported form. Submit a branch-gating allow-list entry both before and after marking its function. Compare occurrence fields and errors with the QSL adapter."}
  - {id: "TC-162 step 13", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "13. Mark one function `#[ix_trace_rs::string_edge]` and one `#[string_edge]` imported from `ix-trace-rs`; scan with the library, the command and the adapter (FR-064-AC-12)."}
  - {id: "TC-162 step 14", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "14. Compile a consumer crate with a marked function and run it; compile a second with `#[string_edge(anything)]` (FR-064-AC-13)."}
  - {id: "TC-162 step 15", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "15. Search the QSL workspace's manifests and production source for a `string_edge` definition, re-export or alias, a `qsl_attrs` path and a `qsl-attrs` member, and resolve each marker (FR-064-AC-14)."}
  - {id: "TC-162 step 16", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "16. Read `cargo metadata` for the QSL workspace and `ix-trace-rs`'s manifest (FR-064-AC-15)."}
  - {id: "TC-162 expected 13", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "Step 13 (FR-064-AC-12): both marked comparisons are absent from every report and the unmarked one is present."}
  - {id: "TC-162 expected 14", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "Step 14 (FR-064-AC-13): the marked function behaves as the unmarked one; the argument case reports exactly the one error."}
  - {id: "TC-162 expected 15", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "Step 15 (FR-064-AC-14): no definition, re-export, alias, path or member is found, and each marker resolves to `ix_trace_rs::string_edge`."}
  - {id: "TC-162 expected 16", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "Step 16 (FR-064-AC-15): `ix-trace-rs` provides `string_edge`, has no normal dependency and no QSL, model, IR or code-generation package, and the layer tables name it as the exempt marker crate."}
findings:
  - fnd: FND-001
    severity: low
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: "TC-162 expected 14"
    related_ids: ["FR-064-AC-13", "TC-162 step 14"]
    path: "spec/test-cases/TC-162-string-edge-scan-coverage.md"
    lines: "182-183"
    excerpt: "Step 14 (FR-064-AC-13): the marked function behaves as the unmarked one; the argument case reports exactly the one error."
    finding: "Step 14's expected result says the argument case 'reports exactly the one error' but does not give the message. FR-064-AC-13 requires the message '`#[string_edge]` takes no arguments', so a test written from TC-162 alone would accept any single error. The AC's backticks are also ambiguous: inline code inside quotes, or part of the message? The existing qsl-attrs message, which ix-trace-rs would take over, is `#[string_edge] takes no arguments` with no backticks. Put the exact message text, without Markdown formatting, in both the AC and the expected result."
  - fnd: FND-002
    severity: low
    confidence: medium
    method: gap-analysis
    check: test-intent
    artifact_id: "TC-162 step 15"
    related_ids: ["FR-064-AC-14", "TC-162 expected 15"]
    path: "spec/test-cases/TC-162-string-edge-scan-coverage.md"
    lines: "90-92"
    excerpt: "15. Search the QSL workspace's manifests and production source for a `string_edge` definition, re-export or alias, a `qsl_attrs` path and a `qsl-attrs` member, and resolve each marker (FR-064-AC-14)."
    finding: "Step 15 says 'resolve each marker' but gives no method. The scan is syntactic, so no textual search shows what `#[string_edge]` resolves to. Name the method: a clean `cargo check` of every QSL package, together with step 15's search showing that no other `string_edge` definition or `use ... string_edge` from another path exists. Without it, two testers would check AC-14's 'resolves to `ix_trace_rs::string_edge`' clause differently."
  - fnd: FND-003
    severity: low
    confidence: medium
    method: gap-analysis
    check: coverage
    artifact_id: "TC-162 step 9"
    related_ids: ["TC-162 step 13", "FR-064-AC-12"]
    path: "spec/test-cases/TC-162-string-edge-scan-coverage.md"
    lines: "62-65"
    excerpt: "9. Exercise the fixtures from steps 1–5 through the public library and binary with consumer rules. Add qualified `#[ix_trace_rs::string_edge]` coverage and the imported form. Submit a branch-gating allow-list entry both before and after marking its function. Compare occurrence fields and errors with the QSL adapter."
    finding: "Step 9 now asks for the qualified `#[ix_trace_rs::string_edge]` form 'and the imported form' through the library and binary. Step 13 asks for the same two forms through the library, the command and the adapter, and is the only step mapped to AC-12. That is two steps for one obligation. Keep the two forms in step 13 only, and leave step 9 on consumer rules and branch-gating entries."
bindings: []
```

+++
