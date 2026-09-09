---
id: TM-006
title: "Native Boolean lowering matrix"
type: TestMatrix
---

## Overview

Scoped to [FR-009](../functional/FR-009-lower-qualified-projections.md).
Five lowering tests and all eight generated truth assignments pass. The required
activation lane fails because the existing codegen reader refuses LLVM 3.1.0;
TC-094 and full FR-009 acceptance remain incomplete. PR reviews are pending.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-009 | FR-009-AC-1 | TC-092 | 🚧 Tested; PR gate pending |
| FR-009 | FR-009-AC-2 | TC-093 | 🚧 Tested; PR gate pending |
| FR-009 | FR-009-AC-3 | TC-092 | 🚧 Tested; PR gate pending |
| FR-009 | FR-009-AC-4 | TC-092 | 🚧 Tested; PR gate pending |
| FR-009 | FR-009-AC-5 | TC-094 | ⛔ Activation pending |
| FR-009 | FR-009-AC-6 | TC-093 | 🚧 Tested; PR gate pending |
| FR-009 | FR-009-AC-7 | TC-093 | 🚧 Tested; PR gate pending |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-092 | Complete binding and correspondence | Integration | P1 | FR-009 | 🚧 Tested; PR gate pending |
| TC-093 | Unsupported forms and bounded work | Integration | P1 | FR-009 | 🚧 Tested; PR gate pending |
| TC-094 | Actual generated truth and activation through IT-008 | Integration | P1 | FR-009 | ⛔ Activation pending |
