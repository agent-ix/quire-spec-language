---
id: SR-13684
title: "failure-domain review of IR-942 ADR-013 O-07/O-09 per-step record identity (PR #1157)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@da13209b734d509e800eda6951e70c4619d9e9d2; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-13684: failure-domain review of IR-942 ADR-013 O-07/O-09 per-step record identity (PR #1157)

## Summary

Ticket: IR-942. Identity confusion, unstated failure modes and edge cases of
the per-step record occurrence key and content identity.

Sound parts: excluding the occurrence key loses no information, because the
key is a function of `(refinement, obligation)` and a repeated pair is
refused, so two records cannot share content and differ only in key. The
tagged two-member preimage cannot collide with the three untagged O-09
preimages, whose member sets differ. Ordinals `i + 1` stay far below 2^53
for a `u32` row position. Hypothesis changes change the digest, as FR-468-AC-5
requires.

Examined: the O-07 amendment paragraph (owner, role, ordinals, ties,
refusals), the O-09 amendment paragraph (preimage, exclusions, refusals),
FR-322 `source_map` totality ("Every node has one or more exact source
occurrences"; regions per occurrence), QSV FR-123 Behavior 1, FR-146
`hypotheses` rule, FR-375 `steps` tie and `memory` rules.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `Initial` record's occurrence (owner, `generated`, 0) derives from no step row, and FR-322's `source_map` is the total occurrence-to-region map. The paragraph says only that "the `source_map` region remains location evidence". It does not say whether these occurrences enter the owner's `occurrences` array and `source_map` at all, nor which region (if any) ordinal 0 carries, nor that a step occurrence's region is its row's. Two implementers would emit different `source_map` entries. State whether the per-step occurrences are `source_map` entries, and give each ordinal's region (for example the refinement header for 0, the row for `i + 1`). | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:254-270 |
| FND-002 | medium | The refusals have no named actor or code. O-07 says "QSL refuses" an unknown refinement, out-of-range row or role/ordinal mismatch. O-09 says "A package refuses" an out-of-range row or a repeated `(refinement, obligation)` pair "using the existing O-07/O-09 typed refusal vocabulary". But O-07's table names only "IR refuses a source-map entry naming an unknown node", and O-09's names only CG's per-`request_index` disposition, so no existing code covers these cases. The companion CG spec has CG refuse "with a typed setup cause". Name who refuses each case (QSL at emission, the FR-322 reader, or CG at setup) and the code and cause each uses. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:266-270, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:313-316 |
| FND-003 | low | "`hypotheses` remain in their supplied order" is correct for QSV, but the paragraph does not say who supplies the order. The digest changes with hypothesis order, so its stability when invariant clauses are reordered in source rests on FR-146's "ascending by node-key digest with no duplicate" rule, which the ADR does not cite. The O-07 paragraph claims source-permutation invariance only for the key. Cite FR-146's ordering rule as the supplier. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:309-311 |
| FND-004 | low | The O-07 paragraph restates part of the FR-375 `steps` order (ties on node-key digest and member name broken by alias-free spelling) but leaves out the rule that `memory` rows sort after every declaration-naming row and tie by memory kind name. A partial restatement next to the authoritative citation invites a reader to apply only the restated part. Drop the restatement and keep the FR-375-AC-19 citation, or restate the whole rule. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:259-261 |

## Verdict

Mergeable on this method's findings once FND-001 and FND-002 are settled;
FND-003 and FND-004 are wording. The identity design itself has no collision
or information loss.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 6dc640fa4: "Each generated occurrence has a `source_map` entry: `Initial` names the refinement declaration region, and `Step` names its corresponding authored step-row region." This matches QSpec #322 (initial region = the node's `declaration` occurrence region; step region = its source row's). |
| FND-002 | fixed | 6dc640fa4: CG refuses an out-of-range row, a duplicate pair, a mismatched owner or role/ordinal at harness setup with `InvalidIdentity` (an existing `KaniErrorCode`) and a typed cause; the IR reader refuses a malformed owner occurrence or `source_map` pair with FR-322's `invalid_package`/`invalid-value`, as QSpec #322 states. A leftover "A package refuses" sentence (line 316) precedes the named actors but no longer leaves the code open. |
| FND-003 | fixed | 6dc640fa4: "`hypotheses` are supplied by QSL in FR-146's ascending node-key digest order without duplicates; CG preserves that order and does no sorting or deduplication." |
| FND-004 | fixed | 6dc640fa4: "For a protocol subject, QSpec FR-375-AC-19 places `memory` rows after rows that name a declaration, then breaks ties by the UTF-8 bytes of the alias-free step-class spelling." |
