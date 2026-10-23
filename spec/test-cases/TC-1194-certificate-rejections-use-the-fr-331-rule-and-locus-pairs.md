---
id: TC-1194
title: "Certificate rejections use only the rule and locus pairs of FR-331"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: verifies
---
# TC-1194: Certificate rejections use only the rule and locus pairs of FR-331

## Description

Verify that QSL writes a `certificate-rejected` cause only with the
spellings and the rule and locus pairs that QSpec FR-331's Certification
table lists. A pair the table does not list is QSL's internal fault and is
never written. Each checker names only the loci FR-338's Rejection
spellings table assigns to it. Scope: FR-338-AC-12 to FR-338-AC-14.

## Test Procedure

1. Write an outcome document whose items settle `CertificateRejected` with
   each FR-338-AC-12 pair, and validate every written cause against QSpec
   FR-331's schema at `QSPEC_DIR`.
2. Write each FR-338-AC-13 unlisted pair, then each of its listed pairs.
3. Read the two FR-338-AC-13 plugin result entries through FR-290's
   reader.
4. Run the rejected vectors of FR-338-AC-2, AC-3 and AC-8, FR-314-AC-2 and
   AC-4, and FR-169-AC-10 and AC-11. For each rejection, compare its locus
   alternative with the checker's row of the Rejection spellings table and
   its rule with FR-331's rules at that locus form.

Tag the tests `#[trace("TC-1194", "FR-338-AC-n")]`.

## Expected Results

- Step 1: the spellings FR-338-AC-12 states, `step/3`, `step` and
  `{"part":"step/4","index":"41"}` included; the schema admits each cause.
- Step 2: each unlisted pair returns `InternalFault` (`runtime_invariant`)
  with invariant `certificate-rejection-pair`, and the document holds no
  `certificate-rejected` cause for it. Each listed pair is written.
- Step 3: `Unsupported` for both entries.
- Step 4: every rejection's locus belongs to its checker's row, and FR-331
  lists its rule at that locus form.
