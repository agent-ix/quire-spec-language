# QSL-656 fix-round dispositions

This note accompanies the unchanged reviewer SR artifacts for the fix round.
It records the planner's disposition separately; it does not alter or replace
the reviewers' exact-head findings.

- **FND-001 — fixed in this PR.** FR-255 again lists `format.output_bytes`
  and states its `FormatLimits` mapping, caller-setting behavior and applicable
  acceptance coverage. The existing TC-931 trace index now also includes
  FR-255-AC-3. The implementation's format-setting/table consistency check is
  restored.
- **FND-005 — downstream adaptation, not a QSL merge blocker.** The quire-driver
  adaptation is tracked by IR-684, which is blocked by QSL-656 and resolves QSL
  by revision pin. The sequencing clarification is recorded in IR-684 comment
  `b8db63b4-2e1e-4d36-a87f-604f734749c5`. No quire-driver source is changed
  here, and this disposition does not claim that the downstream adaptation is
  complete.

The SR files beside this note are copied unchanged from the reviewer artifacts.
Turing's exact-head disposition round remains the next review action.
