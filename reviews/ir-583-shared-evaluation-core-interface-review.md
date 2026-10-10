# Independent review — IR-583 shared evaluation-core design

**Disposition: PASS.** Reviewed frozen commit `ab27a9d556dd667de8795a654138e225b7ad197e` on `design/shared-evaluation-interface`. Scope was the added `spec/assurance/AD-001-shared-evaluation-core-interface.md` and its `spec/spec.md` index entry. No author files were modified; no builds were run.

## Review methods and evidence

Applied the Quoin SpecReview base checklist and the applicable integrity, dependency, failure-domain, scope-boundary, and architecture-evaluation lenses. Quire validation of the new AD exited 0; it emitted only existing module/catalog warnings. `git diff --check` was clean.

Checked the design against FR-262 (including AC-1 through AC-9), FR-294, ADR-030, current `qsl-eval`/`qsl-semantics` source, QSL-682's merged contract, Linear relations for IR-583 and IR-585–IR-593, and RT AD-004 at the cited commit `d610131`.

The design accurately keeps IR-583 as the ready interface/reference-adapter seam and separates it from the no_std S6 implementation. The blocker table matches the current structured graph and planner correction: IR-585, IR-586, IR-587, and IR-589 each require IR-269 and IR-583; IR-588 also requires IR-586; IR-590 requires IR-269, IR-583, and every S1–S5 slice. It does not stage S6 before those prerequisites.

The interface and ownership description follows FR-262: checked authority remains attached to inputs; registration is bound to the admitted closure, identity, and signature; admission precedence and registry sealing are specified; session state and generated-body child handoffs remain owned by the shared core; synchronous QSL entry points drive the same session. The evaluator uses work fuel and explicit heap state, with no depth cap. The 100,000-call / 512 KiB stack and measured-fuel denial/restoration acceptance is correctly distinguished from a fixed-cap smoke test.

The RT section distinguishes the std interface/reference adapter from the IR-590 no_std implementation and requires the governed target, deep behavior, differential results, and port removal before claiming complete migration. It explicitly leaves RT's AD-004/FR-273 amendment to RT and does not claim its target acceptance has run.

## Findings

None. In particular, the use of “superseded” for RT AD-004's older call-depth design is scoped by the same section's requirement for an RT-owned amendment before RT claims migration; I found no material cross-repository authority claim in the frozen text.
