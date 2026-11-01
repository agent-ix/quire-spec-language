---
id: TC-1281
title: "Witness entry names preserve node identity and escape transcript delimiters"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-1281: Witness entry names preserve node identity and escape transcript delimiters

## Description

Verify that the supported replay-root entry-name function produces the
canonical structural node spelling with only transcript-significant bytes
escaped, preserves identity through witness parsing, and leaves entry order
based on the structural node identities rather than their display strings.

## Test Procedure

1. Construct a `NodeKey` whose package/path text contains `%`, `=`, `;`, `|`,
   `<`, `>`, a DEL byte, a quote, a backslash and a non-ASCII scalar. Obtain
   the compact node spelling independently from the shared node-wire codec,
   then call `qsl_replay::witness_entry_name` through the crate root.
2. Construct two keys whose source text differs as `=` versus `%3D`. Call
   `witness_entry_name` for both. Put each name in a one-entry assertion
   transcript with the Boolean value `1`, parse it as a `Witness`, and decode
   it against its original `WitnessBinding`. Also parse a two-entry witness
   using the generated names for two keys with ordinals 2 and 10, in
   structural `NodeKey` order.
3. Attempt to decode the ordinal-2 and ordinal-10 entries in the opposite
   order.

## Expected Results

- The compact node spelling's `%`, `=`, `;`, `|`, `<`, `>` and DEL bytes
  appear in the entry name as `%25`, `%3D`, `%3B`, `%7C`, `%3C`, `%3E` and
  `%7F`, respectively. Other bytes, including the quote, backslash and
  non-ASCII UTF-8, are preserved exactly as they occur in the canonical
  compact spelling. No entry name contains `=`, `;`, `|`, `<` or `>`.
- The two distinct source identities produce distinct entry names, and each
  parsed entry decodes to the value for its matching original `NodeKey`.
- The ordinal-2 and ordinal-10 entries decode in structural `NodeKey` order
  even though their escaped compact JSON names sort in the opposite lexical
  order. Reversing those entries refuses `DecodeRefusal::EntryOrder` naming
  the first out-of-order entry.
