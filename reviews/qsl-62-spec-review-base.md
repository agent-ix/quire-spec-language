---
id: SR-1193
title: "Spec review of quire-spec-language PR #588 commit 6ffdca6f (QSL-62): FR-355 check_fences, TC-901, US-036"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6ffdca6fe18c174773ef1b36e335bcc4cd7a33f2; PR #588 commit 6ffdca6f against main 2ece5712"
review_set: subset
---
# Spec review of quire-spec-language PR #588 commit 6ffdca6f (QSL-62)

## Summary

Ticket: QSL-62. Commit 6ffdca6f adds FR-355 (`check_fences`), TC-901 and US-036, with index rows. The checking path is real: `present` on a non-`Option` operand refuses with a type mismatch at the `present` node (qsl-semantics/src/check/check/typing.rs:1722), as AC-2 and TC-901 step 3 expect. Use-case trace: US-036 `exercises` FR-355 and FR-355 `implements` US-036; EX-1 is AC-1's first clause and EX-2 is AC-2's `present` and `nickname` cases. AC-to-TC trace: AC-1 to AC-5 map one to one onto TC-901 steps 1 to 5, and the tests.md row lists all five. `tools/check-index-completeness.sh` passes.

Examined:
- FR-355 (examined)
- FR-355-AC-1 (examined)
- FR-355-AC-2 (examined)
- FR-355-AC-3 (examined)
- FR-355-AC-4 (examined)
- FR-355-AC-5 (examined)
- TC-901 (examined)
- US-036 (examined)
- ADR-029 LC-1 (context_only)
- FR-277-AC-1 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-355 adds a library operation, `check_fences`, and cites ADR-029 LC-1 for it. LC-1's Check row lists only `select` and `check`, OP-1's operation table has no `check_fences` row, and FR-275-AC-3 and FR-277-AC-1 test "each of the eleven operations", naming them. So the new operation is outside the request, limits, cancellation and determinism tests every lifecycle operation gets, and nothing says how the `check` verb's request selects it. Either add `check_fences` to ADR-029 LC-1 and OP-1 (with its request and the `check` verb's selection) and to FR-275/FR-277's operation lists, or make fence checking a mode of the existing `check` operation. | spec/functional/FR-355-check-the-quire-fences-of-spec-artifacts.md:22-28 |
| FND-002 | low | US-036's So-that names three errors: comparing a string with an enum, `present` on a required field, and an undeclared field. QSL-62 leads with the enum case. AC-2 covers the last two and a parse error, but no AC compares an enum-typed field with a string literal. Add that clause to AC-2 and TC-901 step 2, with its expected type-mismatch refusal. | spec/functional/FR-355-check-the-quire-fences-of-spec-artifacts.md:94 |

## Verdict

Two findings. FND-001 (medium): `check_fences` is a new library operation that ADR-029's operation tables and FR-275/FR-277's "eleven operations" do not include. FND-002 (low): the enum-versus-string error that US-036 and QSL-62 lead with has no acceptance criterion. Not mergeable until FND-001 is fixed.
