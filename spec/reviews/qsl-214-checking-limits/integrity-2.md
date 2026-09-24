---
id: SR-616
title: "Second integrity and EARS review of NFR-011 after the PR #390 review"
type: SpecReview
analysis: integrity
scope: "NFR-011 and TC-423 as revised for the PR #390 review (leaf key bytes charged to work, work default derived from the byte ceiling), the FR-093 reference and the spec/tests.md TC-423 row"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-423
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---

## Summary

I re-ran the base checklist, integrity and EARS over the revision, against
the code. The statement still follows EARS's unwanted-behaviour pattern. The
metrics table gives the new work default, and the counter definitions add
the leaf key-byte charge that `LeafWalk::append` makes.

The derivation of the work default now rests on a fact about the code. The
preimage bytes are checked (`check_input_bytes`) before the write count is
charged to the meter (`family.rs`). The derivation also states that the
cumulative work ceiling can still bind before any one declaration's byte
ceiling, and gives the arithmetic for when it does.

The cost section now separates the bound the limits enforce, which is on
counts (leaves and key bytes), from the memory figures, which are measured.
The TC-423 row now maps each metric to the step that shows its refusal or
admission. FR-093 now cites NFR-011 and no longer restates a number.

Three wording gaps were found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The work derivation said a declaration within the byte ceiling "never reaches the work ceiling on its own". A declaration's lowering also charges work: its nodes and its leaf key bytes. So the claim was true of its preimage writes, not of the declaration. Fix: say that a declaration's writes never reach the work ceiling before its preimage reaches the byte ceiling. | NFR-011 Default derivation; `qsl-semantics/src/check/lowering.rs` `LeafWalk::append` |
| FND-002 | low | The memory figures (41 bytes per key byte, 3.4 KB per leaf, "about 1 GB at most") are extrapolated from two measured shapes. They are not bounds that a limit enforces. Fix: present them as measured rates, and state the enforced bound as the two counts. | NFR-011 Cost at the defaults |
| FND-003 | low | The old text still said the eight-record cluster was within "16%" of the node ceiling. It uses 95904 units, which is 96%. Fix: name the largest input by node units and by work separately, each with its measured share. | NFR-011 Default derivation |

## Resolution

All three findings are fixed in NFR-011:

- FND-001: the work derivation now speaks of the declaration's writes.
- FND-002: the cost section states the enforced counts first, then the
  measured rates, labelled as measured.
- FND-003: the largest recorded inputs are named per ceiling. The
  eight-record cluster is at 96% of the node ceiling, and the
  8000-function chain is at 1% of the work ceiling.
