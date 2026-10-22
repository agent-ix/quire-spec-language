---
id: SR-15456
title: "failure-domain review of STD-261 FR-122 refusal order (QSL PR #1211)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@f2331483313b4bc808636afafed19f23eb301b77; spec/functional/FR-122-replay-a-state-clause-counterexample.md, spec/test-cases/TC-517-replay-a-state-clause-counterexample.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
# SR-15456: failure-domain review of STD-261 FR-122 refusal order (QSL PR #1211)

## Summary

Ticket: STD-261. Examined: the two new refusals that run before the recompile (the target kind
and the envelope/request target mismatch), where they sit relative to FR-098's byte-provision rule and the
recompile, and the lead defaults. The target kind refuses `invalid_runtime_input`/`invalid-value`, which is
sound: the request is well formed under FR-323 but wrong for this entry. The envelope/request mismatch
refuses `stale_dependency`/`content-mismatch`, which is sound: it follows ADR-017 PF-4's clause_node
precedent for two carriers that disagree.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | major: the two new refusals have no stated order. Each FR-122-AC-8 case with a `function` or `frame_operation` request target over the `state_clause` envelope meets both conditions, because the target is the wrong kind and also differs from the envelope's. The AC expects `invalid-value`, but no SHALL picks that refusal over `content-mismatch`. Their position relative to FR-098's byte-provision rule, which also runs before the recompile, is also unstated. AC-8 varies only a source that does not compile. State the order, for example: target kind, then envelope/request mismatch, both before the byte-provision rule (or after it, as chosen). Add an AC-8 case with a missing provision entry. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:227 |

## Verdict

Not mergeable: FND-001 (major). The lead defaults for the codes are sound.
