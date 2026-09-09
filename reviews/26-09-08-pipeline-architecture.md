---
id: SR-074
title: "Architecture evaluation of decoding and model construction boundaries"
type: SpecReview
analysis: architecture-evaluation
scope: "src/, tools/fixture-audit/, tests/ and new native producer at 47985080b9c7077e96cf919eb2a967b98d8a1751"
review_set: subset
---

## Summary

The new fixture producer has a demonstrated provenance defect and combines too
many construction responsibilities. Related orchestration problems occur in the
linker and two historical audit entry points; the existing grammar components
already have clear owners.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | named_locus selects the first matching spelling. NodeRef already appears as a parent-field type reference before its record declaration, so the current fixture gives that declaration the wrong semantic locus despite valid byte coordinates. Repeated/escaped names and reformatting also break selection. Carry a borrowed parsed occurrence and map its checked original byte span. | tests/support/native_rule_model.rs:89; tests/support/native_rule_model.rs:145; tests/fixtures/native-rule-model.json:27 |
| FND-002 | medium | from_text mixes intake, decoding, identity assignment, validation and all lowering. Separate types/scalars maps describe the same scalar identity and can drift during extension. Use focused typed lowering and one scalar binding table; propagate setup errors to the harness boundary. | tests/support/native_rule_model.rs:119 |
| FND-003 | medium | link combines inventory limits/canonicalization, import matching and clause construction in one entry point. Adding the reviewed native profile risks copying admission/resolution policy. Extract shared stages before extending it; preserve existing diagnostic precedence. | src/linking.rs:241; FR-015; FR-017 |
| FND-004 | medium | roles::audit mixes selected-file loading, source-region checks, model/run composition checks and rendering; checkpoint::syntax mixes fixture validation, case conversion, parser execution and rendering. Isolate those stages so contract changes do not require editing one long mixed function. No current wrong audit verdict is demonstrated. | tools/fixture-audit/roles.rs:79; tools/fixture-audit/checkpoint.rs:53 |

## Scenarios and scope

Reviewed immutable source intake, Logos tokenization, Pratt parsing, source maps,
formal linking, JSON audit intake, role/checkpoint/review orchestration and test
helpers. Followed valid input, duplicate identity, malformed/foreign source,
resource exhaustion, success after refusal and addition of another native model
profile. The source-search defect is demonstrated by the checked-in fixture;
the orchestration findings are maintainability/change risks, not invented runtime
failures or a numerical architecture score.

Source::read's NUL search enforces an intake rule. Test searches for deliberately
unique expected markers construct independent controls; they do not assign model
identity. Resolver collection lookups operate on typed declarations. Those are
different responsibilities from reconstructing a declaration locus by spelling.
The existing bounded Serde visitor owns audit JSON grammar admission, and the
native parser already consumes Logos tokens. Exhaustive matches on typed enums
are appropriate conversion/dispatch, not evidence that another lexer is needed.

FR-017 specifies the repair before implementation. Agent A owns this compiler
worktree only. B/C/TL/Filament changes, new semantic services, global skill edits
and unrelated parser rewrites are outside this evaluation. No extra agents or
heavy builds were used. The full model/checker implementation is still absent
at this baseline and the earlier red test run remains explicit.

## Verdict

FAIL at the evaluated baseline. Resolve FND-001–004 through reviewed FR-017 and
record actual code/Rust review and regression outcomes before claiming repair.
