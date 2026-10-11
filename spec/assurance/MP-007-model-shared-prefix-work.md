---
id: MP-007
title: Model shared-prefix work qualification
type: MeasurementPlan
status: proposed
owner: QSL/QSpec model normalization owners
metric: model_identity_shared_prefix_work_units
definition_version: qsl.qspec.model-prefix-work-v1
stage: baseline
subject_identity:
  name: quire.model.complete/v2 identity normalization
  version: ordered-commitment-v2
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: measures
  - target: ix://agent-ix/quire-specification/FR-150
    type: references
---

# Model shared-prefix work qualification

## Decision Use

Determine whether prefix reuse has linear admitted work for the required
10,000-edge chain while preserving exact model identity preimages. This plan is
proposed and is not a release gate or evidence that the full-size run has
completed.

## Status and Location

Keep this plan proposed until the replacement producer, strict reader, selected
closure, and executable measurement apparatus are available together. The
leader owns launching the locked build/gate after review; this authoring task
does not run it.

## Objective

Measure the deterministic shared-prefix work-unit schedule separately from
hashed bytes, ordering/index events, wall time, and peak memory. For an
E-edge single-inheritance chain, verify that fact-run, cycle-check,
normal-form segment/link, and range/binding-proof charges are at most
`12 × (E + 1)`, while all logical facts and eager identities remain present.
At E=10,000 the semantic fact count is 50,015,001; use explicit admission
limits rather than changing the supported bound or suppressing facts.

## Population

The population is the deterministic synthetic family of valid single-inheritance
chain inputs at 10,000 edges, plus smaller ladder and diamond controls used to
verify exact expanded semantics. The required full-size member is capped at
512 KiB of canonical package bytes; no sample may omit identities or logical
facts.

## Collection Procedure

After the integrated v2 implementation and harness exist, run the deterministic
10,000-edge input whose canonical package encoding is at most 512 KiB. Capture
the exact commit, selected definition/rule/accounting closure, input digest,
logical fact count, each named charge subtotal, total work/hashed bytes,
elapsed time, and peak memory. Verify all 10,001 effective type identities and
the complete view. Independently expand smaller ladder and diamond fixtures to
compare fact order, path multiplicity, ordinals, and the exact v2 preimage
bytes/digests. Confirm the published identity-vector artifacts are unchanged.
Repeat the full-size run at least three times for runtime/memory observations;
the deterministic charge bound itself is checked exactly, not statistically.

## Interpretation

Passing the charge bound establishes only amortized accounting behavior. It
does not by itself establish runtime or memory feasibility, transport
round-trip correctness, or release readiness. Report those results separately;
do not infer full-size qualification from extrapolation or the charge bound.
