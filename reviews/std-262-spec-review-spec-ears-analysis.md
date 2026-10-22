---
id: SR-15491
title: "EARS review of STD-262 FR-290 new statements (QSL PR #1217)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@8e62c04067d50955f5388df4071fb2077f0a16b2; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-15491: EARS review of STD-262 FR-290 new statements (QSL PR #1217)

## Summary

Ticket: STD-262. This review covers the five new Behavior statements of FR-290. Each names its subject
(the readers, a reader, `request`) and has one `shall`. Four are ubiquitous and one is
event-driven/unwanted ("When ... `request` shall refuse"). The gate
(`QSPEC_DIR=… make check-spec-validation`) exits 0 and gives no grammar warning on FR-290, TC-774 or
ADR-013. The defects below are about content, not grammar.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: "Each reader shall refuse a member it does not accept, in the order steps 1 to 8 give" contradicts the spec's own text for two of the four readers. `read_plugin_certification` reads a string: step 1 ("The member is not a JSON object: `Malformed`") would refuse the accepted `"trusted"`, against FR-290-AC-7. `read_plugin_assignments` refuses `MissingCounterexample` "before any other check", and its `type` sits at `/counterexample/type`, not `/<member>/type`. So the steps do not describe how it reads. Limit the step 1–8 statement to `read_plugin_basis` and `read_plugin_inconclusive_cause`, and give certification (absent, not a string, not `trusted`) and assignments (the counterexample shape, then `values` and the entries) their own ordered statements. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:335-337 |
| FND-002 | low | minor: "shall accept exactly the wire values that the tables in 'Reading a plugin result member' list". Only `basis` and `cause` have tables. Certification and assignments are given in prose, so for two readers the statement points at nothing. Name the tables and the prose paragraphs, or say "that section states". | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:331-333 |
| FND-003 | low | minor: two outcomes are stated only as indicative prose, with no `shall`: a `PluginReplayRefusal` settles the item `failed` naming the plugin, and the diagnostic goes in the outcome document. A new AC (FR-290-AC-14) checks only the refusal kinds and their order. Add one statement for the settlement, or extend AC-14 to check the `failed` record. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:294-295 |

## Verdict

Not mergeable: FND-001 (major), because a normative statement contradicts AC-7. FND-002 and FND-003
are minor.
