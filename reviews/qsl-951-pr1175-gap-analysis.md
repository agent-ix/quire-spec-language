---
id: SR-13743
title: 'Gap analysis of PR #1175: FR-064-AC-12 to AC-15 against TC-162'
type: SpecReview
analysis: gap-analysis
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

Ticket: QSL-951. PR quire-spec-language#1175, head 76a302455. Planless gap analysis of the four new ACs against TC-162. Plan completion: not assessed. The PR is spec-only, so no test is tagged to FR-064-AC-12 to AC-15 yet, and the code still uses `#[qsl_attrs::string_edge]` (qsl-replay/src/request.rs, witness.rs) and the `qsl-attrs` member. That is expected under the ticket's spec-first order and is not a finding. `quire matrix` was not run.

Each new AC has one procedure step and one expected result in TC-162: AC-12 has step 13, AC-13 step 14, AC-14 step 15 and AC-15 step 16. TC-162's scope covers AC-1 to AC-15, and the TC verifies FR-064. AC-12 can fail: an unmarked function is reported, and both marker forms must be suppressed through all three entry points. AC-13 can fail: behaviour equality and a single compile error. AC-14 can fail: a grep-shaped absence check. AC-15's untestable halves are recorded in SR-13740 FND-003. The findings below are about how precisely the procedures pin those outcomes.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-064-AC-12 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-13 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-14 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-15 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| TC-162 scope | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 step 9 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 step 13 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 step 14 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 step 15 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 step 16 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 expected 13 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 expected 14 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 expected 15 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 expected 16 | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Step 14's expected result says the argument case 'reports exactly the one error' but does not give the message. FR-064-AC-13 requires the message '`#[string_edge]` takes no arguments', so a test written from TC-162 alone would accept any single error. The AC's backticks are also ambiguous: inline code inside quotes, or part of the message? The existing qsl-attrs message, which ix-trace-rs would take over, is `#[string_edge] takes no arguments` with no backticks. Put the exact message text, without Markdown formatting, in both the AC and the expected result. | spec/test-cases/TC-162-string-edge-scan-coverage.md:182-183; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:240; spec/test-cases/TC-162-string-edge-scan-coverage.md:88-89 |
| FND-002 | low | Step 15 says 'resolve each marker' but gives no method. The scan is syntactic, so no textual search shows what `#[string_edge]` resolves to. Name the method: a clean `cargo check` of every QSL package, together with step 15's search showing that no other `string_edge` definition or `use ... string_edge` from another path exists. Without it, two testers would check AC-14's 'resolves to `ix_trace_rs::string_edge`' clause differently. | spec/test-cases/TC-162-string-edge-scan-coverage.md:90-92; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:241; spec/test-cases/TC-162-string-edge-scan-coverage.md:184-185 |
| FND-003 | low | Step 9 now asks for the qualified `#[ix_trace_rs::string_edge]` form 'and the imported form' through the library and binary. Step 13 asks for the same two forms through the library, the command and the adapter, and is the only step mapped to AC-12. That is two steps for one obligation. Keep the two forms in step 13 only, and leave step 9 on consumer rules and branch-gating entries. | spec/test-cases/TC-162-string-edge-scan-coverage.md:62-65; spec/test-cases/TC-162-string-edge-scan-coverage.md:85-87; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:239 |

## Verdict

Mergeable on this lens once the base review's FND-003 (AC-15) is fixed. Three low findings tighten the procedures.
