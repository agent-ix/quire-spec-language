---
id: TC-1197
title: "Certificate rejections use only the rule and locus pairs of FR-331"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: verifies
---
# TC-1197: Certificate rejections use only the rule and locus pairs of FR-331

## Description

Verify that QSL writes a `certificate-rejected` cause only with the
spellings and the rule and locus pairs that QSpec FR-331's Certification
table lists. A pair the table does not list is QSL's internal fault and is
never written. Each checker names only the loci FR-338's Rejection
spellings table assigns to it. The state-graph checker rejects a malformed
certificate at a State or State-graph entry locus. The probability and zone
checkers reject with the rule and locus their tables give each failure.
Scope: FR-338-AC-12 to FR-338-AC-14, FR-169-AC-14, FR-201-AC-6 and
FR-245-AC-7.

## Test Procedure

1. Write an outcome document whose items settle `CertificateRejected` with
   each FR-338-AC-12 pair, and validate every written cause against QSpec
   FR-331's schema at `QSPEC_DIR`.
2. Write each FR-338-AC-13 unlisted pair, then each of its listed pairs.
3. Read the two FR-338-AC-13 plugin result entries through FR-290's
   reader.
4. Check each FR-169-AC-14 malformed state-graph certificate with
   `check_state_graph`, counting recomputed successors, settle it, and
   validate the written cause against FR-331's schema.
5. Check each FR-201-AC-6 probability certificate with
   `check_probability_certificate`, and each FR-245-AC-7 zone certificate
   with `check_zone_certificate`; settle and write each rejection.
6. Run the rejected vectors of FR-338-AC-2, AC-3 and AC-8, FR-314-AC-2 and
   AC-4, FR-169-AC-10, AC-11 and AC-14, FR-201-AC-6 and FR-245-AC-7. For
   each rejection, compare its locus alternative with the checker's row of
   the Rejection spellings table and its rule with FR-331's rules at that
   locus form.

Tag the tests `#[trace("TC-1197", "<criterion>")]` with the criterion each
step verifies.

## Expected Results

- Step 1: the spellings FR-338-AC-12 states, `step/3`, `step` and
  `{"part":"step/4","index":"41"}` included; the schema admits each cause.
- Step 2: each unlisted pair returns `InternalFault` (`runtime_invariant`)
  with invariant `certificate-rejection-pair`, and the document holds no
  `certificate-rejected` cause for it. Each listed pair is written.
- Step 3: `Unsupported` for both entries.
- Step 4: `Malformed` at the locus FR-169-AC-14 gives each certificate,
  with no successor recomputed; the closure-index one is written
  `{"entry": "<length>"}` and the foreign-domain one as a State record in
  a foreign domain; each settles `inconclusive`,
  `CertificateRejected`, never `proved`.
- Step 5: the rule and locus FR-201-AC-6 and FR-245-AC-7 give each
  certificate, written in FR-331's `{product_key}`, `{member}`, `{node}`,
  `{edge}`, `{component}` or State form; the schema admits each cause.
- Step 6: every rejection's locus belongs to its checker's row, and FR-331
  lists its rule at that locus form.
