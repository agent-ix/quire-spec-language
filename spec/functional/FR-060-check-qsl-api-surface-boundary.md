---
id: FR-060
title: "Check the QSL API-surface boundary reachable from backend and internal callers"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: traces_to
---
# FR-060: Check the QSL API-surface boundary reachable from backend and internal callers

## Description

ADR-011 §7.1 T-12 and ADR-013 O-04/O-05 each name a caller boundary that a
crate-level dependency check (FR-059) cannot see, because the boundary is
about which *module*, not which *crate*, is allowed to call a given
construction site:

- CG's normal dependency on QSL (the FR-059 FB-05 exception) is only sound if
  CG's own source calls into QSL exclusively through the layer-6 `replay`
  facade module, never another QSL module.
- The kernel `NodeKey` constructor (ADR-013 O-04, today at its pre-kernel
  location) is only called by the `check` stage.
- The kernel `EffectiveId` constructor (ADR-013 O-05, today at its pre-kernel
  location) is only called by the `model` stage.
- The kernel `PopulationId` constructor (ADR-013 O-13 Population row, QC-21;
  `quire-exact`'s `PopulationId` mint constructor) is only called by the `model`
  stage (ADR-011 §7.1 T-12(d)).

The `quire-exact` kernel crate is ADR-011 §6.1 layer K
([#213](https://github.com/agent-ix/quire-spec-language/issues/213) S-1). This
requirement checks each rule against its *current* call-site location, so the
rule set is live data #213 updates in place as each constructor moves into the
kernel, rather than a check that is retired and rewritten.

## Inputs

- A rule set: for each rule, the textual call pattern(s) that identify a call
  to the constructor or facade the rule protects, the module path prefix(es)
  a caller is allowed to have, the file path the rule's target currently
  requires to exist (so a rule whose target has not landed yet is
  distinguishable from a rule whose target exists and is violated), and the
  *role* whose source tree the rule scans (see below).
- The source tree the rule's role names: QSL's own tree for a QSL-role rule,
  or a caller-supplied CG checkout for a CG-role rule (#249 review, HIGH-2/
  MEDIUM-4).

## Outputs

- Per rule, one of: **live and passing** (the rule's target exists and every
  call site outside the allowed caller modules is absent), **live and
  failing** (the rule's target exists and at least one call site outside the
  allowed caller modules exists; each is reported with file and line), or
  **pending** (the rule's target -- a file the rule requires -- does not
  exist yet, so the rule reports this explicitly rather than passing
  vacuously).

## Behavior

### Rule set (data, not hard-coded per-call logic)

The check SHALL hold its rules as data (one entry per rule: id, description,
call patterns, allowed caller module prefixes, a required file path, a role
and a pending-reason string), so #213 can update the call patterns, allowed
prefixes and required path in place when `quire-exact` lands, without
changing the check's evaluation logic.

Each rule has a **role**: `Qsl` (scan QSL's own tree, the tree already
supplied for the pending-path check) or `Cg` (scan a separately supplied CG
checkout). T12-A is CG-role: its target, the layer-6 `replay` facade module,
is *consumed* by CG, and a violation of "CG calls into QSL only through
`replay`" is only visible by scanning CG's own source, never QSL's (#249
review, HIGH-2/MEDIUM-4 -- evaluating a CG-side rule against the QSL tree
made it impossible for T12-A to ever fail, since QSL's own tree contains no
CG call sites to find). T12-G is QSL-role: it protects constructors QSL itself calls.

At the time of this requirement, the rule set SHALL contain exactly these
seven rules:

| Rule | Role | Protects | Allowed callers (today) | Requires |
| --- | --- | --- | --- | --- |
| T12-A | Cg | The layer-6 `qsl-replay` facade crate | CG's `replay` adapter module; a call spelled `quire_spec_language::replay::` is a violation from any module, because the root crate has no `replay` module, and so is any reference to `qsl_replay::spine`, the spine compile `command` shares with the executor, which is not part of the facade: found by parsing, as a `use` tree leaf or a path through the crate or an alias of it, a glob import of the crate, or a `qsl_replay::spine` token run inside a macro | `qsl-replay/src/lib.rs` |
| T12-B | Qsl | **RETIRED (the `NodeKey` constructors it guarded no longer exist).** | none | none |
| T12-C | Qsl | **RETIRED (the `EffectiveId` constructors it guarded no longer exist).** | none | none |
| T12-D | Qsl | **RETIRED (the `PopulationId` constructors it guarded no longer exist).** | none | none |
| T12-E | Qsl | **RETIRED (the witness minter and `verify_binding` are deleted; a package is identified by `LibraryName`).** | none | none |
| T12-F | Qsl | **RETIRED (the `NodeKey::decode_admitted` decoder it guarded no longer exists).** | none | none |
| T12-G | Qsl | The checked-term constructors: every function of the `quire_checked::construct` module (ADR-011 FB-14, [FR-400](FR-400-separate-checked-terms-from-the-checker.md)), compiled only under the `construct` feature that only `qsl-semantics` enables | `qsl-semantics`' `check` and every descendant module; `value::definition` for the `AdmittedIntegerDivision` constructor | the `construct` module's source in the resolved `quire-checked` checkout |

A rule's call patterns SHALL include every textual spelling that references
the protected item. T12-G's pattern is any path through `quire_checked::construct`, as a `use`
tree leaf, a call or a function value; a crate alias or glob import of the
module counts as a reference.

### Pending vs. live vs. failing

If a rule's required file path does not exist in the source tree under
inspection, the check SHALL report that rule **pending**, naming the missing
path, and SHALL NOT scan for call sites under that rule.

If a rule's role names a tree that was not supplied (a CG-role rule given no
CG checkout, once its target exists), the check SHALL report that rule **not
evaluated**, naming the flag the caller must supply, and SHALL still evaluate
and report every other rule. A run with a not-evaluated rule SHALL exit as a
usage error, with the full report, rather than reporting a vacuous pass.

A QSL-role rule's tree is every QSL workspace crate that depends on
`quire-exact` and does not define the constructors: the root crate and each
extracted layer crate. The shared `no_std` leaves `quire-exact` and
`quire-semantic-value` have their own repositories and are not scanned;
`agent-ix/quire-semantic-value` FR-060-AC-5 and FR-060-AC-6 (clippy
`disallowed-methods`) replace this scan for that crate. If a rule's required file path
exists and its tree was supplied, the check
SHALL scan every `.rs` file under each such crate's `src/` for the rule's call
pattern(s) and SHALL report **failing**, naming each call site's file, line
and enclosing module, for every call site whose enclosing module path is not
exactly one of the rule's allowed prefixes or a descendant of one (matched on
a `::`-segment boundary, so `model_query` is not treated as inside `model`).

A rule with no call site outside its allowed callers SHALL report **passing**.

### Scanning method and its stated limitations

T12-G SHALL match its pattern against the source
file's tokens, not its text. A comment or doc comment is not a token and a
string literal is a single token, so a pattern named inside either never
matches. A call split across lines, a constructor passed as a function
value, a qualified-path spelling (`<NodeKey>::<mint>`) and a call
inside a macro invocation's arguments all match. The rule scans
shipped code only: items under `#[cfg(test)]` are excluded, including a file
declared out of line as `#[cfg(test)] mod name;`, and so is a pattern match
inside a comment or a doc comment.

T12-A scans CG's source text for its literal patterns, so a match inside a
comment or string literal is reported (a possible false positive), not
excluded.

No rule resolves an import alias of the protected item; a call reached
only through a renamed import is a known limitation of this check, not a
silent pass. The check's own report SHALL state these limitations (#249
review, MEDIUM-5).

### The facade's compile entry

The layer-6 `qsl-replay` facade that T12-A protects SHALL carry a compile
entry beside `replay`: `qsl_replay::compile_package(source, package_name, path, bytes,
packages, dependencies, limits, replay_limits)`, with `source` a `SourceIdentity`, `package_name` the package manifest's top-level name, `path` the
display path, `bytes` the source, `packages` the domain package documents
(the same input, in the same position, as `call_site` takes), `dependencies` a
`DependencyInput` and `limits` the `StageLimits` a replay request carries and `replay_limits` the `ReplayLimits` (`replay.input_bytes`) as `replay` takes it. It returns a `CompiledPackage` whose
`bytes()` are the emitted `quire.checked-package/v2` bytes, or a `ReplayRefusal`. It
runs the one S1-to-E4 spine run that `replay`'s recompile runs, under the
stage limits `replay` reads from a request (S1 `text_input_bytes`, S3
`work_units`), and it refuses as that recompile does. A consumer that builds
checked-package bytes from source of its own, such as CG testing against real
QSL output, calls this entry and never names `qsl_replay::spine`.

### T12-A and the evaluator differential corpus

T12-A protects the `qsl-replay` facade, the QSL crate through which CG
calls into QSL; its call patterns name only `qsl_replay`. A crate from its own
repository is not a QSL crate for T12-A, even when QSL owns it: the
evaluation leaf `quire-eval`, the evaluator corpus crate
`quire-eval-corpus` and the checked-term leaf `quire-checked` (all in
`agent-ix/quire-eval`), and `quire-exact`, `quire-semantic-value` and
`quire-walk`. CG's edges to those crates are outside T12-A, and FR-059
governs them.

CG's evaluator corpus producer
([FR-402](FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md))
needs no T12-A change: it writes cases with `quire-eval-corpus`, and it
takes each case's package bytes from `qsl_replay::compile_package` inside
CG's `replay` module, T12-A's existing allowed caller.

### Honest reporting of real findings

Running the check against QSL's real source tree at any commit SHALL report
whatever it finds, including any real call
site the check finds that ADR-011 §1's text does not enumerate. The check SHALL NOT
be tuned to exclude a known finding, or widened to admit an unwanted one, to
make a run pass (#249 review R1).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-060-AC-1 | A rule whose required path does not exist reports `pending` with the missing path named, and contributes no call-site scan. | Test (TC-157) |
| FR-060-AC-2 | A rule whose required path exists and has no call site outside its allowed callers reports `passing`. | Test (TC-157) |
| FR-060-AC-3 | A rule whose required path exists and has a call site outside its allowed callers reports `failing`, naming the call site's file, line and module; a caller module that is a textual prefix but not a `::`-segment descendant (for example `model_query` under an `model` allow-list) is not treated as allowed. | Test (TC-157) |
| FR-060-AC-4 | Run against real QSL source at head with no CG checkout, rule T12-A reports not evaluated, naming `--cg` (its target `qsl-replay/src/lib.rs` exists), the run exits as a usage error, and the remaining live rules are still evaluated and reported. **RETIRED for T12-B, T12-C and T12-D (the constructors they guarded no longer exist).** | Test (TC-157) |
| FR-060-AC-5 | `qsl_replay::compile_package` compiles FR-092's recursive `List` (`record List { next: List?; }`) and `Tree` (`record Tree { kids: Sequence<Tree>[0, 3]; }`) from source through the facade alone, and for each the package bytes equal those of the spine's parse, select, check and package run over the same source. | Test (TC-908) |
| FR-060-AC-6 | `qsl_replay::compile_package` refuses as the spine does: a malformed source refuses `ReplayRefusal::Recompile` with the spine refusal's catalog code and stage; a library whose source has the unit's owner refuses `ReplayRefusal::DependencyInput`; an S1 `text_input_bytes` limit below the source's size refuses `stage_limit_exceeded`; and an S1 limit above the reader limit refuses `LimitAboveReader`. | Test (TC-908) |
| FR-060-AC-7 | `qsl_replay::compile_package` takes the domain packages a unit's `model` selection names as I1's package input: a unit selecting a package compiles, with the package supplied, to the package bytes of the spine's run over the same source and packages, and without the package refuses `missing_import` at the intake stage as the spine does. | Test (TC-908) |
| FR-060-AC-8 | Over a CG fixture tree with a QSL tree whose `qsl-replay/src/lib.rs` exists, a `qsl_replay::compile_package(..)` call in `src/replay/evaluator_corpus.rs` leaves T12-A passing; the same `qsl_replay::compile_package(..)` call moved to `src/oracle/mod.rs` fails T12-A, named with file, line and module. | Test (TC-1113) |

## Dependencies

- ADR-011 §7.1 T-12 and §6.1 layer K
  (`ix://agent-ix/quire-spec-language/ADR-011`).
- ADR-013 O-04, O-05, the O-13 Population row (QC-21) and OBS-018
  (`ix://agent-ix/quire-spec-language/ADR-013`).
- [#213](https://github.com/agent-ix/quire-spec-language/issues/213) owns the
  `quire-exact` kernel crate and updates this rule set's targets once it
  lands.
- [FR-059](FR-059-check-backend-dependency-direction.md) establishes the
  crate-level FB-05 exception this requirement's T12-A rule verifies at the
  module level.
