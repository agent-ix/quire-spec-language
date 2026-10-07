---
id: SR-1384
title: "Gap analysis of quire-spec-language PR #664 (QSL-654): ProofBound domain kind and admit domains"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6bcd0259772fe70eabc0274ce916bc0ebc5e1aa3; PR #664 diff against merge base 1372b151 (origin/main bfeb258c); QSL spec FR-057, FR-097, FR-288, ADR-013 C-28, ADR-029 PV-4 (origin/main); QSpec ada3f9eb FR-290, FR-331"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: reviews
---
# Gap analysis of quire-spec-language PR #664

## Summary

Ticket: QSL-654. Planless. I read the computed matrix with `quire matrix --format tsv`, quire 0.36.1, at the head. Plan completion: not assessed.

Examined:
- QSL FR-057 registration paragraph (lines 246-251), its implementation note (lines 414-416) and FR-057-AC-8 (examined)
- QSL FR-097 Outputs and FR-097-AC-3, AC-4 (examined)
- ADR-013 C-28 as changed by the PR (examined)
- ADR-029 PV-4 item 1 on origin/main bfeb258c (examined)
- QSpec FR-290 registration paragraph and AC-13; FR-331 extent and manifest `domains` (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No QSL requirement owns the `invalid-domains` refusal. QSL FR-057 still says the registry "SHALL refuse ... (`absent-kind`, `unknown-kind` or `unknown-mode`)", and its implementation note lists the same three causes. FR-057-AC-8 covers only those three. The code adds a fourth cause, and the new tests have no QSL criterion to bind to (see SR-1383 FND-003). | spec/functional/FR-057-admit-shared-capability-kinds.md:246-251; spec/functional/FR-057-admit-shared-capability-kinds.md:414-416; qsl-route/src/lib.rs:392-416 |
| FND-002 | medium | No QSL requirement owns `ProofBound.kind`. FR-097's Outputs and AC-3 say only that a bounded item carries "its proof bounds" in key order. Neither says that each bound carries its domain kind, paired as FR-331 states, so the writer could drop `kind` and no FR-097 criterion would fail. | spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:53-54; spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:71; qsl-route/src/request.rs:266-270 |
| FND-003 | medium | Once this PR merges, ADR-029 PV-4 item 1 on main (from #661) is false. It says CG's descriptor is "not `qsl_route::BackendDescriptor`, which holds only (kind, mode)", but after the PR `qsl_route::BackendDescriptor` also holds the admitted `domains`. The PR does not touch ADR-029. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:488-490; qsl-route/src/lib.rs:218 |
| FND-004 | low | The reworded C-28 pins its source as "QSpec FR-290 @74645130", a commit SHA in prose (QSpec #196's merge commit; the reviewed QSpec main is ada3f9eb). The repo's CLAUDE.md forbids manual SHA bookkeeping. The pin goes stale the next time FR-290 changes, and nothing resolves it. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1030 |

## Verdict

**Changes requested (medium).** The code covers every case in QSpec FR-290-AC-13's registration sentence, and the routing.rs tests exercise each one. The QSL spec has not caught up: FR-057 and FR-097 do not state the new behaviour, so it has no QSL acceptance criterion, and ADR-029 PV-4 contradicts it after the merge. The domain-kind routing step of FR-290-AC-13 is CG's, as already decided, and is not counted as a QSL gap here.
