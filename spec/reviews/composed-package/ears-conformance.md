---
id: SR-307
title: "EARS conformance of composed compiler requirements"
type: SpecReview
analysis: ears-conformance
scope: "Requirement statements in FR-035 and FR-036"
review_set: all
evaluated_revision: "fa07b07"
---
## Summary

Reviewed the twenty-three requirement statements in FR-035/036 at `fa07b07`.
The deterministic engine reports both documents grammar-clean; semantic review
found no trigger, subject or measurable-response defect in these statements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No EARS defect found: each requirement statement names its compiler/parser/linker actor, has one SHALL and states an observable syntax, binding, refusal or provenance response. | FR-035; FR-036 |

## Engine evidence

Quire is `0.31.0` (CLI `4f6ed024`, engine `0.46.0@ca7362d4`). The scoped command
was `quire validate --scope /home/peter/dev/worktrees/quire-language-composed-spec
'spec/functional/FR-035-parse-composed-native-units.md'
'spec/functional/FR-036-link-composed-native-packages.md' --summary`.
It exited 0: 2/2 documents grammar-clean, zero grammar findings. The installation
also reported duplicate archetype/inverse-edge contributions with first-wins
selection; those are module-loading notices, not EARS findings in these files.
The separate 6/14 property-extractable count is not execution or EARS coverage.

## Semantic assessment

The two Description events select an edition and start explicit-inventory
linking. They describe request events, not continuous operating states.
Eleven ubiquitous statements define preservation, syntax classification,
namespace resolution and retained dependencies. Ten unwanted-condition
statements use If/then for malformed, unavailable, conflicting, cyclic,
unsupported and resource-exhausted inputs. No When phrase disguises a refusal.

Responses are inspectable: original half-open spans, typed declaration variants,
exact selected definition/model ownership, complete requested inventories and
explicit unfinished/refused dispositions. Collections of retained identity
fields are one preservation obligation rather than multiple contradictory
responses. API naming remains an implementation choice without making output
types opaque or the acceptance oracle subjective.

The actor named parser does not reassign every diagnostic to Phase::Parse:
`src/parser.rs::header` already emits Code::UnknownEdition at Phase::Profile,
and source/lexical failures precede syntax construction. FR-035 preserves
historical behavior. FR-036's linker establishes dependencies before
`checking::check`; its linked report does not claim a checked Boolean or family
execution. These stage distinctions make the observable responses assessable
without assuming an unimplemented engine.

US stories, TC procedures, IT-009 and informative context were inspected for
interpretation but are outside the requirement-statement EARS population.
This lens does not replace the integrity review of package agreement and
resource-accounting prerequisites.

## Correction recheck

Targeted semantic reread of `d5047a8` found the added FR-036 edition-agreement
statement uses the unwanted-condition pattern correctly: If disagreeing source
headers, then the named linker refuses namespace admission with the conflicting
selections and loci. It adds one unwanted-condition statement to the original
twenty-three. The accounting clarification gives concrete inputs and observable
charge/refusal rules; it introduces no vague SHALL response or competing actor.
The original engine evidence above remains attributed to the baseline check,
not to a new full review or execution run.

## Verdict

PASS for the scoped EARS lens at the evaluated revision and the targeted
semantic correction recheck at `d5047a8`. Standard acceptance, implementation
and execution evidence remain outstanding; no new build or test run is claimed.
