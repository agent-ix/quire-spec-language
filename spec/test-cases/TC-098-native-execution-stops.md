---
id: TC-098
title: "Preserve native execution stops and fresh retries"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-023
    type: verifies
---
## Description

Retain failed request provenance and distinguish validation from evaluation stops.

## Test Procedure

Offer malformed populations and foreign authored selections. Lower validation
work and evaluation step/event limits. Cancel at each stage and retry with fresh
budgets. Compare the combined report with actual separate validate/evaluate runs.

## Expected Results

Failed validation starts no evaluation. Original diagnostics, terminal reasons,
ordered event prefixes and counters match the existing stage APIs. A Boolean
exists only on completed evaluation; later retries are independent of earlier stops.
