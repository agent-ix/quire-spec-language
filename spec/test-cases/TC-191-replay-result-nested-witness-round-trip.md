---
id: TC-191
title: "A replay result's nested witness record round-trips exactly, compares without display-text interpretation, and refuses an oversized encoding"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: verifies
---
# TC-191: A replay result's nested witness record round-trips exactly, compares without display-text interpretation, and refuses an oversized encoding

## Description

Verify that a decisive `Witness`-arm replay result's construct → serialize
→ read round trip preserves the nested FR-351 separating-witness record's
deciding element, index, value path and trace position exactly; that
comparing two results for agreement reads only these typed fields, never a
rendered transcript or a diagnostic message string; and that a result
whose encoded size exceeds the reader's configured bound refuses rather
than decoding a truncated result. A wrong implementation this test would
catch: an agreement check implemented as `result_a.to_string() ==
result_b.to_string()` (or an equivalent pretty-printed comparison) instead
of a structural comparison of the typed FR-351 fields; two results with
the same rendered text but different value paths (for example, differing
only in which collection member decided the result) would then incorrectly
compare equal; or a reader that streams and returns a partially-decoded
result once it exceeds the size bound instead of refusing the whole
construction. Scope: FR-072-AC-3, FR-072-AC-5.

## Test Procedure

1. Construct a decisive `Witness`-arm replay result whose FR-351 record
   names a specific deciding element, a nonzero index, a multi-segment
   value path (into a nested record and a collection), and a trace
   position.
2. Serialize and read the result back; compare the FR-351 record's four
   fields, field by field, against step 1.
3. Construct a second result identical in every FR-351 field except the
   value path (a different nested member decided it), but whose rendered
   debug/display text happens to be textually identical to the first.
4. Compare the two results from steps 1 and 3 for agreement using the
   type's own comparison, and separately confirm no display/rendering
   method is invoked during that comparison (by code inspection or an
   instrumented render call that would panic/log if invoked).
5. Construct a result whose encoded size exceeds the reader's configured
   bound (pad the resolved-regions or charges members with repeated valid
   entries until the bound is crossed), and inspect whatever value the
   constructor returns.

## Expected Results

- All four FR-351 fields are identical, byte-for-byte or value-for-value,
  between the constructed and read-back results in step 2.
- The two results in step 4 compare as **not equal**, because their value
  paths differ, despite having textually identical rendered output.
- No display/rendering function is invoked by the comparison in step 4.
- Step 5 refuses with a bound-exceeded cause; the constructor returns no
  result value at all, in particular no result holding a truncated prefix
  of the padded entries.
