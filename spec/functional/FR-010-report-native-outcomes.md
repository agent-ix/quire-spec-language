---
id: FR-010
title: "Report phase-specific CLI outcomes"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: implements
---
# FR-010: Report phase-specific CLI outcomes

## Description

When the native CLI finishes a request, it shall report the observed pipeline outcome with its source identity.

## Inputs

Command, source labels/file and compiler outcome.

## Outputs

JSON parse/diagnostic output or formatted source plus a documented exit code.

## Behavior

Current parse reports parsed only. Exit codes are 0 for successful syntax, 1 refusal, 2 usage/I/O failure and 3 incomplete resource exhaustion. Source diagnostics preserve original byte and scalar coordinates. Future link/evaluate outcomes cannot be inferred from parse success.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-010-AC-1 | A successful parse emits status parsed. | Test |
| FR-010-AC-2 | A refused syntax request exits 1. | Test |
| FR-010-AC-3 | An invalid command invocation exits 2. | Test |
| FR-010-AC-4 | An exhausted parser request exits 3. | Test |
| FR-010-AC-5 | A parse result carries the actual source digest. | Test |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [Detailed contract or implementation evidence](../../src/main.rs) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
