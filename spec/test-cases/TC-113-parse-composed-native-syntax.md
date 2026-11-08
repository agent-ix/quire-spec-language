---
id: TC-113
title: "Recognize composed syntax without changing historical grammar"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: verifies
---
# TC-113: Recognize composed syntax without changing historical grammar

## Description

Exercise the public parser against the composed grammar and the historical
corpus. Steps 1 to 4, 6 and 7 verify the retired FR-035-AC-1 to AC-6 and are
retained as migration history: their composed-unit parser is deleted, and the
lossless CST parser (FR-002) with the S2 form builders (FR-308, FR-257) owns
located syntax. Step 5 verifies the active FR-035-AC-7: the `1-draft`
declarations are parsed and checked by the spine front end.

## Test Procedure

1. Construct a complete four-declaration unit from the standard's order/refund
   example with explicit edition, profile and model selections. Inspect each
   declaration variant and every cross-family expression's actual syntax tree.
2. Compare trees for implication associativity, arithmetic precedence and grouped
   temporal relations. Distinguish a temporal constant from `holds(true)`.
3. Mutate one form at a time: chained comparison/relation, temporal formula in a
   value argument, missing protocol operand, truncated compensation and trailing
   garbage. Require a diagnostic rather than a recovered successful unit.
   Select an unavailable edition in an otherwise valid header and require the
   typed unknown-edition diagnostic at that selection, with no successful unit.
4. Use `M::result` and `view.retry` in qualified model positions, then use those
   spellings as native binders. Compare edition-specific keyword treatment with a
   valid historical identifier newly reserved by the composed edition.
5. For each exact spelling `helper`, `rec`, `cast` and `Tuple`, use it under
   `1-draft` as a function name (and call it), a predicate name, the `using`
   profile alias of each, a parameter, a `let` binder, a record name and its
   field, an enum member, a type-alias name and a model selection alias (each
   unit also declaring a function), and
   as a qualified `M::<spelling>` type and `x.<spelling>` member access.
   Confirm each unit parses and checks with no `invalid_syntax` and no
   `unsupported_construct`, that a call of a function so named evaluates, and
   that a function `using <spelling>` is admitted with its profile alias
   resolved to the profile that `profile <spelling>` selects. Use lowercase `tuple`
   as a record name and confirm its `invalid_syntax`/`unexpected-token`
   refusal. Under `0-draft`, use each of the four exact spellings in a value
   position and confirm the historical `unsupported_construct` refusal is
   unchanged.
6. Insert CRLF, comments, escaped quotes and multibyte string values. Check spans
   by slicing the original bytes at every selected declaration/reference/literal,
   including delimiters, rather than comparing decoded-character offsets.
7. Run exact-limit and one-step-over token/node controls, then the
   historical parse/format/refusal/package corpus through its original selection.

## Expected Results

The composed forms produce typed, located syntax with no linked or checked claim.
Malformed forms return their typed source-bound diagnostics; budget failures are
`stage_limit_exceeded` with the budget's cause, a token budget
`token-count-exceeded`. Under `1-draft`, `helper`, `rec`, `cast` and `Tuple`
are ordinary identifiers in every name position, a call of a function so named
evaluates and its `using` selection is kept; lowercase `tuple` is refused as a
record name; the same four spellings under `0-draft` keep their historical
`unsupported_construct` refusal. All historical expectations remain
byte-for-byte unchanged. Each step has unconditional assertions; a missing declaration or absent
error object fails the test rather than skipping the check.
