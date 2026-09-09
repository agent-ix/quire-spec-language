---
id: TC-073
title: "Retain implication event lineage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-073: Retain implication event lineage

## Description

Integration, priority P1. Verifies FR-008-AC-7, FR-008-AC-8, FR-008-AC-9, FR-008-AC-10, FR-008-AC-17. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Execute false/true, grouped, nested and repeated implications in native LF/CRLF/multibyte sources. Check independent ordered entry/completion events and exact original operand byte/scalar positions and ExprIds. Exhaust expression fuel before an operand's first non-Group node and event storage at each append boundary.

## Expected Results

Events prove only actual operand entry and antecedent completion. False antecedents have no consequent entry. Exhaustion retains the actual prefix and no Boolean; grouping preserves lineage without a step. Event storage cannot silently discard an observation and return completed.
