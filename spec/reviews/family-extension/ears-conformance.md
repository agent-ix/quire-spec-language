---
id: SR-481
title: "EARS conformance review of ADR-012 semantic-family extension contracts"
type: SpecReview
analysis: ears-conformance
scope: "spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/spec.md (ADR-012 relationship and index row)"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
# SR-481: EARS conformance review of ADR-012

## Summary

Reviewed commit `048deb3` (branch `task/210-family-extension`):
`spec/decisions/ADR-012-semantic-family-extension-contracts.md`, its
`contains` relationship (line 39) and its index row (line 388) in
`spec/spec.md`. ADR-012 is an ADR, not an FR, NFR or StR. The engine's EARS
scope is therefore empty, and `quire validate --strict` reports 1/1 docs
grammar-clean with 0 findings. This review applies the EARS intent by hand to
the normative statements: the ten Decision items, the §4 rule and the §4.2
builder rules, the §5 seam and registry tables, §6, §7, §8's two rules and
the §9 rule. It asks one question of each: is there one subject, one trigger
or condition, and one observable response, so that #212 or an implementer can
tell pass from fail?

Most statements pass. §5.1's no-`_` rule, §6's "the checker never reads the
registry" with its byte-identical package, §7.1's registry-as-value rule and
§7.3's absent-capability mechanics are each directly testable. Two statements
are ambiguous enough that an implementer could not tell pass from fail:

- For an unbounded requirement with an available finite bound, §1.1 settles
  `requires-bound`. §7.2 settles `unsupported`, because a bounded-only backend
  is not a candidate and an empty candidate set yields `unsupported`
  (FND-001).
- Decision 6 makes CG `negotiate_*` the single settlement point. §7.2 and
  Consequences say each backend settles through its own negotiator
  (FND-002).

The medium findings are mostly unstated edge conditions: an empty requirement
set, the state after an out-of-order clause, a family with no `evaluate`
hook, how the seam test fails the compile, and a named backend that is not
registered. One is a conflict with an existing requirement: the OBS-003
removal breaks FR-036's traced unsupported-disposition statements. Current-
state wording is sound except for one "no longer" in Consequences. No
compatibility layer is designed: §13.4 Q4 asks the owner a standalone
question and designs no reader, which is correct.

Several statements need to become FRs before implementation starts, because
an ADR row cannot carry a Test Matrix binding. FND-014 lists them: the
registry behaviour before #185, the builder behaviour before #214, and the
absent-capability and solver-absence outcomes before #213. FR-036 must also
be revised before #185 (FND-013).

Verdict: ACCEPT WITH FINDINGS. FND-001 and FND-002 must be resolved before
#212 applies scenario 7 and before #185 or the CG negotiation ticket starts.

## Method

1. Ran the engine check:
   `quire validate --scope /home/peter/dev/worktrees/qsl-arch11 spec/decisions/ADR-012-semantic-family-extension-contracts.md --strict --summary`.
   Result: `1/1 docs grammar-clean (100%); 0 grammar finding(s)`. An ADR
   carries no `shall` statements, so this is expected and says nothing about
   testability.
2. Listed every normative statement in the Decision list, §1.1, §2, §4.1,
   §4.2, §4.3, §5.1 to §5.3, §6, §7.1 to §7.4, §8 and §9. For each one I
   checked the subject, the trigger or condition, whether the response is
   observable, and whether edge inputs are covered. That second step is the
   EARS semantic judgment the engine cannot make.
3. Checked each statement against the records it adopts or constrains:
   #210's Acceptance list, QSpec AD-016 on `origin/main` (terminal-
   disposition rule, arrows 1 to 7, "Frames and unbounded constructs"), #185's
   exit criteria, and the QSL requirements the decisions change (FR-036).
4. Scanned for current-state wording ("no longer", "was", "previously") and
   for any compatibility, fallback or legacy reader.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `requires-bound` cannot be reached under the stated selection rules. §1.1 says a backend that advertises a capability only in bounded mode "is not a candidate" for an unbounded requirement, and that "when a finite bound is available, `negotiate_*` settles `requires-bound`". §7.2 gives an unbounded requirement with only bounded advertisers an empty candidate set, and an empty set settles `unsupported`. `requires-bound` then appears only as the one candidate's own settlement, which needs a backend that advertises (capability, `Unbounded`). AD-016 "Frames and unbounded constructs" maps an unbounded construct to `requires-bound` when a finite bound is available and to `unsupported` otherwise. #189's exit criterion depends on which answer holds. Fix: add a §7.2 table row and a §5.2 row stating that when the candidate set for (capability, `Unbounded`) is empty, the set for (capability, `Bounded`) is non-empty, and a finite bound is available (#222), `negotiate_*` settles `requires-bound`; otherwise it settles `unsupported`, warned. Then make §1.1's third bullet cite that row. | ADR-012 §1.1, §5.2, §7.2 |
| FND-002 | high | The ADR does not say who settles a disposition for a non-CG backend. Decision 6, §7.1 (Negotiator row) and §7.2 prose make CG `negotiate_*` "the single point that settles a disposition". But the §7.2 "exactly one" row says "that backend's own settlement", and Consequences says "a new backend costs one descriptor plus its own negotiator". `BackendDescriptor` (§7.1) has no negotiator field, and Decision 5 claims "four owners" while §6 names two for backend capability (registry and CG). #185, the CG ticket and #212 scenario 7 cannot tell whether a second backend, for example Verus, is settled inside CG `negotiate_*` or through a function the backend supplies. Fix: choose one rule and state it in Decision 6, §7.1, §7.2 and Consequences. Either "each backend adds one `negotiate_<backend>` function in CG, and `negotiate_*` is that closed family of CG functions", or "the descriptor carries a typed negotiator, and CG `negotiate_*` is the only caller". Also change Decision 5 to name the candidate owner and the disposition owner separately. | ADR-012 Decision 5, Decision 6, §6, §7.1, §7.2, Consequences |
| FND-003 | medium | §7.2 step 1 does not define the result for an empty requirement set. A `Requirements` whose `capabilities` set is empty is advertised vacuously by every registrant. With two registrants and no named backend, the result is `invalid-request`. §12.1's "Requirements: none beyond the `Value` family's" makes an empty set plausible. Fix: state either that a requested item's `capabilities` is non-empty by type (a non-empty set in #213), or that an item with no required capability is not a requested item and is neither routed nor negotiated. | ADR-012 §2 Requirements, §7.2, §12.1 |
| FND-004 | medium | §4.2 calls the builder a typestate over clause order and says an out-of-order clause "is a typed refusal raised by the transition it tried to take". A compile-time typestate has no transition for a disallowed pair, and source order is only known at run time, so something must map (state, clause) to a transition at run time. The ADR does not say what that driver is, whether its `match` is a legal thin seam (§4.3), what state the builder is in after an out-of-order refusal, or whether later clauses are still checked. Fix: state that one driver `feed(state, clause) -> Result<state, OutOfOrder>` is the only transition entry. Its `(state, clause kind)` match has an explicit refusal arm for each disallowed pair and no `_` arm. After a refusal the builder keeps its prior state and continues with the next clause. | ADR-012 §4.2, §4.3, §12.2 Tests |
| FND-005 | medium | Two §4.2 rules each allow two readings. "Diagnostics are aggregated in declared clause order" could mean the builder's state order or the source order, and these differ exactly when a clause is out of order. "`finish` runs its cross-clause checks only when every clause they read was checked" could count a clause that was checked and refused. Fix: "diagnostics are ordered by the builder's state order, and within a repeated clause kind by source order", and "`finish` runs a cross-clause check only when every clause it reads produced a checked subnode". | ADR-012 §4.2 |
| FND-006 | medium | The §4.2 state diagram is not marked as normative or illustrative. Read as normative, it allows no `Header → [*]`, `Header → Post` or `Framed → Post` edge. That means every protocol operation needs at least one precondition, and a frame cannot be followed directly by a postcondition. §12.2 depends on this diagram. Fix: label the diagram as the protocol-operation builder and derive its edges from the QSpec operation grammar, or label it illustrative and put the normative clause order in §12.2. | ADR-012 §4.2, §12.2 |
| FND-007 | medium | §2's trait shape declares `evaluate` as a required method, but the text says a family with no reference evaluation (`Relation`) "implements no `evaluate` hook" and no stub. A required trait method must be implemented, and a default body would be the forbidden stub. S1 also lists "reference evaluation dispatch" as a seam over `FamilyKind`, which needs a `Relation` arm, and §8's Replay row reaches "the family's `evaluate` hook". Fix: move `evaluate` into a separate trait that only evaluable families implement. State that the S1 reference-evaluation arm for a family without it returns an explicit typed refusal with a catalog code, and that such a family has no replay row. | ADR-012 §2, §5.1 S1, §8 |
| FND-008 | medium | §5.1 and §5.3 say adding a variant "fails the build at every listed seam", and #214 "shows a compile failure at each of S1–S4". The ADR does not say how a test adds a variant to a production enum or how it checks that each seam, and not only the first, failed. It also does not forbid `#[non_exhaustive]`, and a cross-crate `match` on a `#[non_exhaustive]` enum must have a wildcard arm (relevant to S5, S6 and S8). Fix: name the mechanism. For example, a feature-gated probe variant, and a gate that builds with that feature and asserts rustc E0004 at each listed seam's file and function. Add "none of the S1–S8 enums is `#[non_exhaustive]`". | ADR-012 §5.1, §5.3 |
| FND-009 | medium | §5.2 decides that a capability kind outside the #229 vocabulary is "refused at registration", but §13.3 Q4 asks #229 to decide "unknown-kind behaviour at registration". The row also cannot be reached for an in-process descriptor, because S7 is a closed enum. Fix: limit the row to descriptors decoded from a wire form, state that the decode refuses the unknown kind, and delete Q4. Alternatively, mark the row "decided in #229" and keep Q4. | ADR-012 §5.2, §13.3 Q4 |
| FND-010 | medium | §8's second rule, "an item refused or settled `unsupported` at one stage emits no substitute artifact", is narrower than the AD-016 terminal-disposition rule the record says it adopts unchanged. That rule also covers `requires-bound` and `invalid-request`. Fix: "An item refused, or settled `requires-bound`, `unsupported` or `invalid-request`, emits no substitute artifact at any later stage." | ADR-012 §8, QSpec AD-016 Terminal-disposition rule |
| FND-011 | medium | The §9 rule cannot be checked as written, and its own table breaks it. "Serialization or command-line edge" is not defined, so any string comparison can claim to be at an edge. The rule converts only "into a closed enum", but the `--target` row converts to `BackendId`, an identity over an open set of backends, and two rows key on checked declaration identity. §7.2 also turns a named backend that is not registered into an empty candidate set, so a mistyped `--target` settles `unsupported` naming a capability. Fix: list the edge modules, or a marker attribute, so a lint can find every other string comparison. Allow the conversion target to be "a closed enum or a typed identity". State whether a named `BackendId` that is not registered settles `invalid-request` naming the id, or `unsupported`. | ADR-012 Decision 8, §7.2, §9 |
| FND-012 | medium | Decision 4, second sentence, says adding a semantic variant "at an open seam (backend registration) fails explicitly through the registry". Registering a backend is not adding a semantic variant, and a valid new registration should succeed. The sentence has no testable response. Fix: "At the open seam, a duplicate backend identity, an unadvertised requirement or an ambiguous match yields the explicit §5.2 outcome, never a first-wins or last-wins default." | ADR-012 Decision 4, §5.2 |
| FND-013 | medium | The OBS-003 decision conflicts with FR-036, and §14 does not mention FR-036. The decision removes the composed linker's `Backend` parameter and its `UnsupportedCapability` and `UnsupportedFamily` dispositions. FR-036 line 97 still says the compiler "SHALL retain its typed unsupported disposition", and lines 185–191 assess each pair "against a caller-declared backend" with distinct unsupported-capability and unsupported-family outcomes. FR-036-AC-6 is traced to TC-115. After #185, TC-115 either fails or tests removed behaviour. Fix: add a §14 row that says FR-036 must be revised before #185 starts, so that it records requests as data only and moves unsupported dispositions to CG `negotiate_*`, and cite FR-036 in the OBS-003 row. | ADR-012 §10 OBS-003, §14; FR-036 (lines 97, 185–191, AC-6) |
| FND-014 | medium | Several statements must become FRs before their tickets start, because an ADR row cannot carry a Test Matrix binding. Before #185: the §5.2 registry failure table, §7.1 order independence (equal registries select identically), the §7.2 candidate table (including the FND-001 and FND-003 rows) and §6 "a checked package is the same bytes whatever backends are registered". Before #214: the §4.2 builder behaviour (out-of-order refusal, diagnostic order, refused construct emits no checked node) and the §4.3 thin-seam rule. Before #213: the §7.3 absent-capability outcome (warning naming kind and mode, never a refusal, success or hold) and the §7.4 solver-absence mechanics (probed only at execute or prove, no fallback, no downgrade), with #229 supplying codes. The §5.1 and §5.3 compile-failure obligations can stay as ADR test obligations. Fix: add a §14 row, or a "Requirements to author" subsection, naming each FR and its owning ticket. | ADR-012 §4.2, §4.3, §5.2, §6, §7.1–§7.4, §14 |
| FND-015 | low | §4.3's "every arm makes exactly one call into the owning family and holds no semantic logic of its own" does not say whether wrapping the result in the output enum's constructor, or propagating it with `?`, counts as logic. Fix: "each arm's body is one call expression into the owning family, optionally wrapped in a constructor of the seam's output type". | ADR-012 §4.3 |
| FND-016 | low | §7.2's empty-set row and §7.3 say the warning names "the required capability kind and mode" in the singular, but a `Requirements` holds a set of kinds. Fix: "naming every required (capability, mode) pair that no candidate advertises". | ADR-012 §7.2, §7.3 |
| FND-017 | low | In §7.4, "The ticket fixes only that solver absence is never a hold" has no clear referent (#210 or #229). §7.2 says an unavailable backend's item "settles the solver-absence outcome", which reads as a second disposition next to Decision 6's single settlement point. Fix: name the ticket, and state that solver absence is the FR-331 result of an item already settled `supported`, not a second disposition. That keeps AD-016's one-disposition, one-result join on `request_index`. | ADR-012 §7.2, §7.4, Decision 6 |
| FND-018 | low | §10 OBS-014 and Consequences both say "five production string sites", but §9's table has twelve rows across QSL, IR, CG and RT, so #212 cannot tell which five are meant. Fix: mark the five OBS-014 rows in §9, for example with an OBS-014 column, or list them by row in §10. | ADR-012 §9, §10 OBS-014, Consequences |
| FND-019 | low | The current-state rule is broken once, and one normative cell uses a non-binding modal. Consequences says "the checked package no longer varies with installed backends", which narrates a change. §10 OBS-013's decision cell says FR-290's phrase "should say so". Fix: "The checked package does not vary with installed backends", and "FR-290's registrant phrase is amended in QSpec (§13.4 Q1)". | ADR-012 Consequences, §10 OBS-013 |
| FND-020 | low | Decision 1 says mode is "a typed axis that every family declares", but §1.1 says "a family does not own a mode" and that each checked node states its mode through `Requirements`. Fix: "a typed axis carried in every checked node's `Requirements`". | ADR-012 Decision 1, §1.1 |
| FND-021 | low | §11's test says a ticket waits on #185 "only when" an exit criterion is registry behaviour, but the table uses the test in both directions (kept and relaxed). Fix: "if and only if". | ADR-012 §11 |
| FND-022 | low | §1's "No family calls another family's checking or lowering internals" does not define "internals", so it can only be checked by reading the code. Fix: state that a family module exports only its checked types and its contract implementation, and that everything else is private to the module, so the compiler enforces the rule. | ADR-012 §1 |
