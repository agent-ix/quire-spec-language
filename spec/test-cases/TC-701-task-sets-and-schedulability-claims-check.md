---
id: TC-701
title: "S3 checks task sets and routes schedulability claims to EN-7"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: verifies
---
# TC-701: S3 checks task sets and routes schedulability claims to EN-7

## Description

Verify task-set checking, refusals, premises in the obligation identity, and routing under the `schedulability` capability kind.

Scope: FR-246-AC-1 to FR-246-AC-2.

## Test Procedure

Fixtures: ADR-026 §12's `Ctl`; its variant with third WCET 6; the refusal fixtures of FR-246-AC-2.

1. Check `Ctl` and route `schedulable Ctl` under fixed priority and EDF; compare obligation identities with the WCET-6 variant.
2. Check each refusal fixture, and the priority clash under `edf`.

Tag the tests `#[trace("TC-701", "FR-246-AC-n")]`.

## Expected Results

- Step 1: checks; both route to EN-7 under `schedulability`; identities differ.
- Step 2: each refuses at its span with the stated code; the `edf` claim checks.
