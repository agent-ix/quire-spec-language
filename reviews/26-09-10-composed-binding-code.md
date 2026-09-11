---
id: SR-319
title: "Code review of composed definition, model and lexical binding"
type: SpecReview
analysis: code-review
scope: "FR-036 / TC-114; src/linking/composed.rs; src/linking/composed/{binding,binding_work,definitions,definition_source,models,scopes}.rs; src/linking/composed/scopes/{values,protocol}.rs; src/linking/composed/scopes/protocol/flow.rs; tests/composed_{binding,definitions,definition_source,models,scopes}.rs; resources/native-v1/; README.md; spec/{spec.md,functional/FR-036-link-composed-native-packages.md,model-linking/tests.md,test-cases/TC-114-bind-composed-package-dependencies.md}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-114
    type: references
---

## Summary

Reviewed `agent-a/composed-native-binding` (92313b7) against
`agent-a/composed-native-linking` under `code-review` + `rust-review` +
`rust-style`, plus the repo's own `AGENTS.md`/`CLAUDE.md` idioms. The binding
design is sound — charge-before-work is real, dispositions never fabricate empty
success, scope traversal is iterative, and the exactness/unsupported boundaries
the spec claims are actually enforced. One high finding: the per-declaration
arena rescan is quadratic and makes a legal package unbindable at the
unraisable hard reference ceiling.

## Verdict

**FAIL** — FND-001 is high: a well-formed single-unit package inside the
parser's own 50 000-node ceiling cannot be bound at any caller limit setting.
Everything else is medium or low.

## Gates

All run in this session from the worktree, one heavy command at a time under
`flock /tmp/quire-heavy-check.lock` with `CARGO_BUILD_JOBS=1`,
`CARGO_TARGET_DIR=/tmp/formalization-a-language-target`, `nice -n 10`.

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (exit 0) |
| `cargo clippy --locked --all-targets --no-default-features -- -D warnings` | pass (exit 0) |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass (exit 0) |
| `cargo test --locked --no-default-features -- --test-threads=1` | pass — 401 passed, 0 failed, 4 ignored |
| `cargo test --locked --all-features -- --test-threads=1` | pass — 417 passed, 0 failed, 4 ignored |
| `cargo deny check` | not applicable — no `deny.toml` in the repo |

The four `#[ignore]`d tests are pre-existing named lanes documented in
`README.md`, not new. The implementation agent's logs in `/tmp/quire-binding-*.log`
agree with these outcomes; its `quire-binding-fmt.log` and
`quire-binding-first-check.log` record two failures (module ordering, a missing
`flow` file) that were fixed before the commit under review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Per-declaration rescan of the whole unit arena is quadratic; a legal package exceeds the unraisable 2 000 000 reference ceiling and reports Unfinished | src/linking/composed/models.rs:969, src/linking/composed/models.rs:990, src/linking/composed/definitions.rs:263, src/linking/composed/binding.rs:292 |
| FND-002 | medium | Three `unreachable!()` added to recover data the caller already held, one of them forced by a shadowed binding | src/linking/composed/scopes/protocol.rs:570, src/linking/composed/scopes/protocol/flow.rs:94, src/linking/composed/scopes/protocol/flow.rs:110 |
| FND-003 | medium | Frame-descendant invariant is undocumented and unenforced; three walks `.expect()` on it instead of erroring | src/linking/composed/scopes.rs:546, src/linking/composed/scopes/protocol.rs:151, src/linking/composed/scopes/protocol/flow.rs:69 |
| FND-004 | medium | Catch-all `_` arms over closed enums silently absorb a new variant, unlike every other new match in the change | src/linking/composed/models.rs:684, src/linking/composed/models.rs:800, src/linking/composed/models.rs:825, src/linking/composed/models.rs:870 |
| FND-005 | medium | Two near-identical leaf-unwrap loops charge different dimensions, so exact-limit expectations differ by path | src/linking/composed/models.rs:795, src/linking/composed/models.rs:819 |
| FND-006 | medium | Model refusal records are created without the documented Bindings charge; relationship refusals skip `visit()` too | src/linking/composed/models.rs:940, src/linking/composed/models.rs:1063 |
| FND-007 | medium | Malformed revision spelling degrades to `StaleSelection` while a malformed digest gets a typed `InvalidDigest`; plus a `String` allocation per candidate | src/linking/composed/models.rs:330, src/linking/composed/models.rs:376 |
| FND-008 | medium | `BTreeMap` key shapes force a clone per lookup, and `iter().find` scans of unbounded catalogs are billed as one Reference | src/linking/composed/scopes/protocol.rs:353, src/linking/composed/models.rs:429, src/linking/composed/models.rs:642, src/linking/composed/models.rs:686 |
| FND-009 | medium | `pub fn parameter_type` has no caller and no test; `pub const ACCOUNTING_VERSION` for this stage is never asserted | src/linking/composed/models.rs:522, src/linking/composed/binding_work.rs:5 |
| FND-010 | medium | Eleven producible typed causes have no test, including the primary undefined-name diagnosis | src/linking/composed/scopes.rs:260, src/linking/composed/definitions.rs:84, src/linking/composed/models.rs:97 |
| FND-011 | medium | `resources/native-v1/` carries no co-located provenance or licensing record, and `external/quire-spec-language/**` is a byte-identical copy of first-party files with no drift control | resources/native-v1/, src/linking/composed/definition_source.rs:4, src/linking/composed/definition_source.rs:242 |
| FND-012 | low | Six requirement ids preserved under `resources/native-v1/spec/**` collide with local `spec/` ids; only tool scoping convention keeps them apart | resources/native-v1/spec/functional/FR-036-retain-lexical-source-locations.md:2, spec/functional/FR-036-link-composed-native-packages.md:2 |
| FND-013 | low | `Cause::DefinitionCycle` and `Cause::IncompatibleRequirement` are structurally unreachable in the closed registry | src/linking/composed/definitions.rs:86, src/linking/composed/definitions.rs:88 |
| FND-014 | low | Three new submodules omit the `FR-` owning-requirement `//!` header the other six carry | src/linking/composed/scopes/values.rs:2, src/linking/composed/scopes/protocol.rs:2, src/linking/composed/scopes/protocol/flow.rs:2 |

## Detail

**FND-001.** `ModelWalk::visit` is charged for *every* control and *every*
expression in the unit before the span-containment filter decides whether the
node even belongs to the declaration (`models.rs:969-996`), and `bind` calls
`resolve_declaration` once per declaration (`binding.rs:292-311`).
`definitions::resolve` repeats the pattern over `unit.controls()` for each
protocol declaration (`definitions.rs:263-268`). Cost is therefore
`D_u × (C_u + E_u)` References per unit. `HARD_LIMITS.references` is 2 000 000
and `Limits::effective()` clamps *down* only (`binding_work.rs:42-51`), so no
caller can raise it. Scenario: one unit with 50 declarations over a 40 000-node
expression arena — comfortably inside the parser's `nodes` ceiling of 50 000
(`src/syntax.rs:21-24`) — spends exactly 2 000 000 References on the containment
scans alone, leaving nothing for the real work; 100 declarations over the same
arena needs 4 000 000. The result is `Disposition::Unfinished` with a
`ResourceExhausted` cause for input that is well-formed and that the namespace
stage already accepted. The repo already has the right mechanism:
`dependencies.rs::declaration_at` (a single ordered cursor pass per arena,
documented at `dependencies.rs:187-197`) does this attribution in `O(C+E)` per
unit rather than per declaration. Two new sites bypassed it and re-derived
ownership a third way.

**FND-002.** `protocol.rs:559` shadows the `event: &ControlId` bound at line 547
with the resolved `&Control`, then re-matches `control.kind` at 570-573 with
`_ => unreachable!()` purely to get the handle back. `flow.rs:91-95` and
`flow.rs:107-111` re-fetch and re-match a control the scheduler already
classified. All three are unreachable today; all three encode a scheduler
invariant no type enforces. If `Task::Sequence` is ever scheduled for a
non-sequence node the library panics rather than refusing. Carrying the child
list in the `Task` (or renaming the shadowed binding) removes the panic and the
re-match.

**FND-003.** `scopes.rs:544-551`, `protocol.rs:149-156` and `flow.rs:67-72` all
walk `while cursor != base.frame` and `.expect()` that the chain reaches `base`.
That holds only because every `Task::Control` arm in `flow.rs` happens to leave
`result` at a descendant of its inherited environment — a property stated
nowhere and checked by nothing. A future control kind whose arm sets
`result = input` inside a parallel branch turns `merge_branch` into a library
panic. The invariant belongs in a comment at minimum, and the walks should
return an `Exhaustion`-style refusal rather than unwinding.

**FND-004.** `models.rs:684` absorbs every unmatched `NativeType` into
`WrongExportKind`; `models.rs:870` absorbs every unmatched `ModelErrorKind` into
`None`. `models.rs:800`/`825` treat any unmatched `ir::ValueType` as a leaf.
The last is the live risk: `quire-contract-ir` is pinned by rev, and if the next
bump adds another container variant, `formal_type` stops unwrapping early and
`catalog.formal` either mis-types the value or returns `None` and reports a
spurious `WrongExportKind` — with no compile error anywhere. Every other new
match in this change is exhaustive (`definitions.rs:389-396`,
`protocol.rs:622-638`, `flow.rs:138-212`), which is why these four stand out.

**FND-005.** `scalar_type` (`models.rs:795-802`) charges one Reference per
Option/Collection unwrap. `formal_type` (`models.rs:819-827`) charges a
Reference *and* a Binding for the identical step, though no record is created
and `Limits::bindings` is documented as "Created binder, role, export, or
declaration result records" (`binding_work.rs:16`). Binding the same
`Collection<Option<Text>>` field therefore costs different Bindings depending on
which path reached it, which breaks the FR-036-AC-7 property that exact-limit
expectations are derivable from a controlled input.

**FND-006.** `ModelWalk::record` pushes a `ModelError` with no charge
(`models.rs:1063`), and the relationship loop pushes
`UnsupportedRelationshipContract` directly onto `report.refusals` with neither a
`visit()` nor a charge (`models.rs:940-944`). `bind` compensates by charging one
Binding per `Refusal::Model` (`binding.rs:301-310`), but `resolve_declaration`
is `pub` and is called directly by `tests/composed_models.rs`; on that path a
protocol with N relationships retains N records for zero charged Bindings. The
module doc and `binding.rs:176-179` both promise every result/cause costs one
Binding.

**FND-007.** A malformed digest gets a typed `ImportRefusal::InvalidDigest`
before any candidate scan (`models.rs:330-331`). A revision is instead compared
as `owner.revision().get().to_string() == revision` inside the scan
(`models.rs:376`), so `version "007"` against `RequirementRevision(7)` — with a
correct package and digest — reports `StaleSelection` ("No candidate has all
selected revision/digest components"), which is a misleading cause for a
spelling problem. The same line allocates a `String` from a `u64` on every
candidate for every import; parsing the authored revision once, outside the
loop, removes both the allocation and the misdiagnosis.
`tests/composed_models.rs:259-306` covers a genuinely different revision but no
non-canonical spelling of the correct one.

**FND-008.** `Names::children` is keyed `(Option<SymbolId>, String)`, so each
scope level walked clones the path component (`protocol.rs:353`, `:370`);
`ModelBindings::aliases` is keyed `(UnitId, String)` and clones the alias on
every resolution (`models.rs:429`). Neither clone expresses intent — both exist
to satisfy the key type. Separately, `Catalog` is keyed by `&ir::SymbolName`,
which is not `Borrow<str>`, so lookups degrade to `iter().find(...)`
(`models.rs:470-479`, `:642-647`, `:686-691`) while `models.rs:792-793` indexes
the same maps directly. Each linear scan is charged one Reference, so a model
with 10 000 values makes every `value()` lookup a 10 000-entry scan billed as
`O(1)` — the same accounting-honesty concern as FND-001, at smaller scale.

**FND-009.** `ModelBindings::parameter_type` has exactly one occurrence in the
tree: its own definition. `binding_work::ACCOUNTING_VERSION`
(`"composed-binding-work/1"`) is never referenced or asserted, whereas the
namespace stage's counterpart is pinned by `tests/composed_namespace.rs:657`.
FR-036 requires the public boundary to declare its work-accounting version; it
is declared but nothing stops it being renamed silently.

**FND-010.** Producible and untested: `definitions::Cause::{WrongEdition,
AmbiguousRule, MissingAlias}`; `models::ModelErrorKind::{MissingAlias,
AmbiguousExport, IncompleteCatalog}`; `models::ImportRefusal::AmbiguousSelection`;
`scopes::ScopeIssue::{MissingValue, ModelOperationUnavailable, DuplicateSymbol,
InvalidAwaitEvent, IncompatibleReference}`. `ScopeIssue::MissingValue` is the
most consequential: `finish_names` (`scopes.rs:557-576`) rewrites it to
`OutOfScope` whenever the name is bound anywhere in the declaration, and only
the rewrite is tested, so deleting the `MissingValue` arm at `values.rs:41-45`
or making the rewrite unconditional would leave the suite green. Severity is
held at medium because `AGENTS.md` (2026-09-09) explicitly defers assurance
completion to a later campaign; the gap itself is recorded in SR-320.

**FND-011.** `resources/native-v1/` adds 45 files of preserved external
normative bytes with no `README`/`NOTICE` beside them; `README.md` and
`LICENSE-DECISION.md` do not mention the tree. The only provenance and licensing
notice is the module doc at `definition_source.rs:4-13`. The repo `LICENSE` is
AGPL-3.0-only, so anything reading the tree by itself will infer AGPL for
another repository's documents — which is precisely what `AGENTS.md` forbids
("Do not infer licensing from process or repository boundaries"; no relicensing
without approval). Separately,
`resources/native-v1/external/quire-spec-language/src/diagnostic.rs` and
`docs/native-error-codes.md` are byte-identical to the repo's own current
`src/diagnostic.rs` and `docs/native-error-codes.md` and are labelled with URLs
into this same repository at f444d03c, so "external" is inaccurate and the
copies will drift silently the next time a `Code` variant is added. Nothing
compares them. The remedy is a co-located provenance/licensing record and a
drift control — not a per-file checksum catalog, which `CLAUDE.md` prohibits.

**FND-012.** `resources/native-v1/spec/functional/FR-036-retain-lexical-source-locations.md`
declares `id: FR-036`, as does `spec/functional/FR-036-link-composed-native-packages.md`.
FR-030, FR-031, FR-032, FR-033 and FR-035 collide the same way. Nothing in the
repo pins the separation: `quire` reads documents from `<repo>/spec` only, so
today the collision is inert, but it rests entirely on that tool convention.

**FND-013.** With the registry's fourteen identities all distinct and its
`requirements()` graph acyclic (`definition_source.rs:158-250`), the
`identities` collision at `definitions.rs:510-517` and the `active` cycle check
at `definitions.rs:502-506` can never fire. Reasonable defensive code for a
registry that may grow, but it is currently dead and should be labelled as such.

**FND-014.** `values.rs`, `protocol.rs` and `flow.rs` open with a descriptive
`//!` line but no `FR-` id, unlike the six sibling modules and the prior-PR
`dependencies.rs`.

## What holds up

- **Charge-before-work is genuine.** `Work::charge` reserves with
  `checked_add(...).filter(|next| *next <= limit)` and leaves the counter
  untouched on refusal or overflow (`binding_work.rs:132-153`). Every new
  traversal charges before allocating, and `charge_exports` (`models.rs:755-781`)
  reserves the export index records *before* `Catalog::new` builds them.
- **No fabricated success.** `Report::disposition` (`binding.rs:139-170`)
  returns `Unfinished` for any declaration not covered by a complete run and
  never lets a missing component read as empty admission; `ScopeReport`
  keeps `Option<DeclarationScope>` slots for the same reason
  (`scopes.rs:341-361`). Early `Err` returns at `binding.rs:268/289/298/314`
  stop later stages after any component exhaustion.
- **The claimed boundary is real.** No Markdown or JSON is parsed anywhere in
  the registry: definitions and rules are matched by raw-byte equality
  (`definitions.rs:425-437`, `:535`), `schema.json` is embedded as opaque bytes,
  and nothing constructs a model schema. `admits()` (`definitions.rs:377-397`)
  is exhaustive over the registry, so no definition gains a clause role by
  omission.
- **Identity substitution is refused, not coerced.** A resealed artifact with
  matching bytes still yields `UnsupportedDefinition`
  (`tests/composed_definitions.rs:203-233`); foreign-model access is refused by
  requiring both artifact identity and a unit-local alias (`models.rs:586-615`);
  relationship export authority is refused unconditionally
  (`models.rs:940-944`), matching FR-036's explicit unsupported boundary.
- **Lexical and causal scope behave.** Activation exports captures while leaving
  the trigger behind (`scopes.rs:540-555`), choice/repeat/await restore the
  entry environment so nothing escapes a path that may not be taken
  (`flow.rs:152`, `:168`, `:127`), parallel merges every branch because all
  branches run (`flow.rs:122-125`), and `preceding` checks availability against
  the *environment chain* rather than visit order (`flow.rs:33-58`).
- **Tests are substantive.** All 35 new tests carry `#[trace]` tags that resolve;
  none is ignored, `#[should_panic]`, or assertion-free.
  `tests/composed_definition_source.rs` transcribes expectations independently
  and compares embedded bytes against the on-disk resources (reading from disk
  here is correct — `include_bytes!` would make the assertion tautological), and
  `tests/composed_definitions.rs:412-449` derives exact per-dimension limits from
  the input rather than from a recorded run.
- No `unsafe`, no `#[cfg(test)]` branch in production code, no new
  `#[allow(...)]`, no suppressed warning, no new dependency, no filesystem or
  network discovery on the binding path.

## Not findings

- `role.sites[0]` and the `catalog.values[name]` / `catalog.fields[&…]` indexes
  at `models.rs:790-793` are guarded by `NativeModel` admission, which rejects
  empty `sites` and unassigned primitive sites
  (`src/native_model/admission.rs:337-385`).
- `DuplicateBinder` firing across disjoint sub-scopes (`scopes.rs:454-458`) is a
  deliberate no-shadowing rule, consistent with FR-036's "a trigger binder
  unavailable in a later formula cannot be rescued".
- `std::ptr::eq` for model identity (`models.rs:601`) is correct here: identity
  *is* the exact supplied artifact instance.
- The control arenas have no cycle guard, but `ComposedUnit`'s fields are
  `pub(crate)` and only the parser builds them, so no caller can inject one; the
  charged loops would exhaust rather than hang regardless.
