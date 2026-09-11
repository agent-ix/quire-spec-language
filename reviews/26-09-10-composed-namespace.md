---
id: SR-318
title: "Code review of composed namespace and native dependencies"
type: SpecReview
analysis: code-review
scope: "FR-036 / TC-114; src/linking/composed.rs; src/linking/composed/; src/{linking,parser,syntax}.rs; tests/{composed_namespace,composed_linking}.rs"
review_set: subset
---

## Summary

**PASS — no open code/Rust findings.** Reviewed the namespace slice against
`agent-a/composed-native-parser`, FR-036/TC-114 and the code-review/Rust-review skills,
including gap discovery. The namespace test author performed this contributor review;
it is not independent qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings remain after the two corrections below. | src/linking/composed.rs; tests/composed_linking.rs |

Public API documentation now explains index ownership, original loci, stage-only
dispositions and effective limits; the redundant `entries()` getter was removed.
Four excessive AC tags were corrected: native target tests no longer cite model
binding AC-3, expression ownership no longer cites runtime-role AC-4, and the
header test no longer cites historical-artifact AC-8. Assertions are unchanged.

Exact source matching closes the namespace before lookup. Shared conflict groups
retain every declaration locus; separate unit handles preserve arena ownership.
Iterative SCC and refusal propagation retain cycles and causal references without
recursive traversal or copied dependency paths. Charges precede bounded work,
checked addition refuses overflow, and exhaustion cannot expose an `Available`
declaration before dependency completion. Historical header retention changes
neither grammar acceptance nor package serialization. No unsafe code, test bypass,
new dependency, hidden discovery or unowned implementation was found.

The 25 public tests inspect typed outcomes, target/source distinctions and exact
input-derived budgets. Definition/model/type/lexical/role binding and IT-009 remain
explicitly outstanding in FR-036/TC-114; their absence is not concealed by this API.

## Validation

Local gates pass: 367 tests with minimal features and 383 with all features,
including all 25 new tests; four existing ignored tests remain in each suite.
Strict all-targets Clippy passed in both configurations, and rustfmt passes.
Cargo ran serially with one build job and one test thread. Quire reports 381/381
documents grammar-clean with no findings; the staged diff check passes. No
dependencies or CI workflows changed. The reviewer ran no Cargo commands.
