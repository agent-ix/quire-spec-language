---
id: SR-12138
title: "QSL-515 PR #930 delta gap analysis since 49ad03552: FR-110 corrections and the value.complete navigation exception"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@54ff5705807d8ea9820959148d428056f247d0a3; delta 49ad03552..HEAD (non-merge commits 74a01ecc5, d2b72ea75, 31237c084, 18a5d5c77, 1511a7f37 resolution, b79448146, 4b84a2ab5); FR-110-AC-7, FR-110-AC-8, FR-110-CON-1, FR-110 Outputs and Profile resolution bullets, FR-341-AC-6, FR-361 items 6-7; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md, spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, qsl-semantics/src/value/layer.rs, qsl-semantics/src/value/definition.rs, qsl-package/src/emit.rs, qsl-semantics/src/check/layer.rs, qsl-replay/src/spine/clause/tests/layers.rs"
review_set: subset
---

## Summary

Ticket: QSL-515. PR quire-spec-language#930. This is a planless,
delta-scoped gap analysis over `49ad03552..HEAD`. The full analysis is
SR-12064. Plan completion: not assessed. Criteria were traced to tests by
reading. No matrix and no cargo were run (brief: static review plus
`quire validate`). Linear QSL-750 and QSL-827 were read as data to check the
references the spec text makes.

FR-110 corrections checked against code:

- Outputs: the headers' layers reach the checked graph as `header_layers`,
  and the emitter reads them for `definition_selections`. This matches
  `CheckedGraph::header_layers` and `catalog_selections(.., graph.header_layers(), ..)`
  in qsl-package/src/emit.rs. It fixes the stale sentence SR-12064 noted.
- Known limit (QSL-827): `=`/`!=` map to `None` in `expression_form`. This
  is accurate. QSL-827 exists ("Layer classifier: admit equality by operand
  type, not syntax", Backlog).
- No-header default: `catalog_selections` selects `always_roles()` when
  `layers` is empty and no layer row. The bullet matches the code. See
  FND-002 for the conflict with "Lock rows".
- The value.complete exception bullet names `deref` and `reaches`. This
  matches `ExprNode::Deref | ExprNode::Reaches => ObjectNavigation`, and the
  `ValueComplete` row holds `ObjectNavigation`. See FND-001 for the expiry.
- FR-110-CON-1: the deleted clause about a per-row listing test matches the
  removal of those tests in 74a01ecc5.

Bindings examined: FR-341-AC-6 to `a_layer_refusal_is_prohibited` (correct,
now pins the cause). FR-110-AC-8 to `state_core_admits_only_its_own_forms`
(correct for the refusal halves). FR-110-AC-8 to the three wider-layer
`every_form_under` tests, and FR-110-AC-7 to the editor test, are wrong
bindings, recorded in SR-12137 FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The expiry condition of the value.complete `ObjectNavigation` exception does not fall within the ticket it names. value/layer.rs and FR-110 say the exception "expires when QSL-750 (FR-361) lands; that work deletes the `ObjectNavigation` entry". But FR-361 item 7 says QSL "does not gate ... field or relationship navigation". Its only `reaches` sentence defers to FR-110's layer rule. No FR-361 AC and no enumerated QSL-750 item deletes the value.complete row's `ObjectNavigation` entry or moves `deref`/`reaches` units off `quire.value.complete/v1`. The repo rule requires the expiry condition to be written into the ticket, and QSL-750's description does not carry it. As written, QSL-750 can land with the exception still in place, so the temporary exception has no expiry anyone will act on. Fix: add the deletion (and moving the navigation fixtures to `quire.state.graph/v1` or `quire.model.complete/v1`) to QSL-750 as an enumerated item with an AC, and to FR-361 or FR-110 as the condition. Alternatively, name a ticket whose scope is that deletion. | qsl-semantics/src/value/layer.rs:196-205; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:115-118; spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:128-138 |
| FND-002 | medium | Two FR-110 statements disagree on the no-header unit. The new bullet says a unit with no header profile "holds the package-selection `always` rows ... and no layer row", and the code does that. "Lock rows" says the emitter writes "its `package_selection.without_layer` rows otherwise", that is, whenever the union does not hold `quire.value.complete/v1`. An empty union does not hold it. Two implementers would read the empty case differently. No AC or test binds the default either, so a regression to `without_layer` would pass. Fix: scope "otherwise" in Lock rows to a non-empty union, and add an AC with a test for a unit with no header profile. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:112-114,174-178; qsl-package/src/emit.rs:847-866 |
| FND-003 | low | FR-110-CON-1 now reads "QSL adds no form of its own". The exception bullet and the value/layer.rs comment say the value.complete set admits a form wider than QSpec's. CON-1 should name the exception or point to the bullet. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:185 |

## Verdict

PASS with mediums. No major bug. The FR-110 Outputs correction, the
QSL-827 known limit, the exception's form list and the CON-1 trim are
accurate against the code. FND-001 is the real gap: the exception is marked
temporary, but the ticket it names neither carries nor implies its expiry.
It is recorded at medium because nothing ships wrong today. The admission is
documented and tested. Left unfixed, the exception becomes permanent.
FND-002 is a spec ambiguity with no test behind the default.
