---
id: SR-323
title: "Code review of the explicit rational native model profile"
type: SpecReview
analysis: code-review
scope: "FR-041 / TC-120; src/native_model.rs; src/native_model/{admission,artifact}.rs; src/model_source.rs; src/model_source/{decode,lower,wire}.rs; src/linking/native.rs; src/linking.rs; src/checking/{types,variables}.rs; src/package/features.rs; src/runtime/validation/values.rs; src/wire_format.rs; tests/native_model_profiles.rs; tests/compile_command.rs; README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-120
    type: references
---

## Summary

Reviewed `agent-a/composed-value-checking` (40f6b43) against
`agent-a/composed-native-binding` under `code-review` + `rust-review` +
`rust-style`, plus the repo's own `AGENTS.md`/`CLAUDE.md` idioms and the pinned
`quire-contract-ir` at 690bde7. The producer slice is correct and well tested:
`/1` bytes are unchanged, the rational wire variant is genuinely closed, bounds
go through the real `ir::RationalType::new` without repair, and every budget is
charged before its work with checked arithmetic. No live defect was found. The
findings are latent-boundary and idiom issues, the most consequential being that
the shared `Catalog::formal` now types a rational as an ordinary nominal scalar
for the *historical* checker as well as for composed binding.

## Verdict

**CONDITIONAL** — no high finding. All five gates pass. FND-001 is the one to
act on before FR-040 work lands: it is unreachable today only because a single
call site refuses `/2`, and behind it a rational would type-check an ordering
comparison and be materialized into the IR proof as a Boolean.

## Gates

All run in this session from the worktree, one heavy command at a time under
`flock /tmp/quire-heavy-check.lock` with `CARGO_BUILD_JOBS=1`,
`CARGO_TARGET_DIR=/tmp/formalization-a-language-target`, `nice -n 10`. Exit
statuses captured directly; no gate piped through `tail`/`head`.

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | pass (exit 0) |
| `cargo clippy --locked --all-targets --no-default-features -- -D warnings` | pass (exit 0) |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass (exit 0) |
| `cargo test --locked --no-default-features -- --test-threads=1` | pass (exit 0) — 433 passed, 0 failed, 4 ignored |
| `cargo test --locked --all-features -- --test-threads=1` | pass (exit 0) — 449 passed, 0 failed, 4 ignored |
| `cargo deny check` | not applicable — no `deny.toml` in the repo |

The four `#[ignore]`d tests are the pre-existing named lanes (three `IT-004`
private-packet, one LC04 activation gate), unchanged by this branch. These
outcomes match the implementation agent's recorded 433/449.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Removing the `Rational => None` refusal makes a rational a nominal scalar for the historical checker too, where ordering type-checks and the proof materializes it as Boolean | src/checking/types.rs:305, src/checking/constraints.rs:514, src/checking/proof.rs:184, src/checking/types.rs:333, src/checking/types.rs:351 |
| FND-002 | medium | Profile identity has three unlinked spellings and the checker's gate is a string comparison that cannot see the model's actual profile | src/linking.rs:202, src/checking/bindings.rs:17, src/native_model.rs:24, src/native_model.rs:42 |
| FND-003 | medium | `ScalarV1` is a hand-maintained duplicate of the `/1` wire variants and no test decodes an integer scalar through both formats | src/model_source/wire.rs:32, src/model_source/wire.rs:68, tests/native_model_profiles.rs:113 |
| FND-004 | medium | The same profile decision is written exhaustively in one place and as `matches!` in another, and three `ScalarKind` sites absorbed the new variant with no compile error | src/native_model/admission.rs:66, src/native_model/admission.rs:165, src/checking/types.rs:122, src/checking/types.rs:132, src/checking/constraints.rs:498 |
| FND-005 | low | The `/2` historical refusal relies on an undocumented positional invariant across two modules, and its own fallback arm is structurally unreachable and untested | src/linking/native.rs:20, src/linking/native.rs:22, src/linking.rs:384 |
| FND-006 | low | The extracted `Variables` leaks one union-find array to expose a length, keeps `Result<_, ()>` causes, and its header claims a sharing that does not exist | src/checking/variables.rs:8, src/checking/variables.rs:39, src/checking/variables.rs:2, src/checking/constraints.rs:444 |
| FND-007 | low | The twelve-mutation refusal table asserts one shared code for six distinct specified causes | tests/native_model_profiles.rs:435, src/native_model/admission.rs:412, src/native_model/admission.rs:428 |
| FND-008 | low | `TryFrom<&str>`/`UnknownNativeModelProfile` have no production caller and diverge from the crate's `FromStr` convention; `ModelDraft`'s retag protection is thinner than the FR claims | src/native_model.rs:37, src/digest.rs:38, src/model_source.rs:205, tests/native_model_profiles.rs:174 |
| FND-009 | low | Two feature-derivation arms return an error no supported input can reach, via `return` inside a `match` used as a call argument | src/package/features.rs:71, src/package/features.rs:112 |

## Detail

**FND-001.** `Catalog::formal` previously ended with an explicit
`ir::ValueType::Rational { .. } => None` refusal. That arm is gone and rational
now joins integer and text in producing `NativeType::Scalar`
(`types.rs:303-310`). `Catalog` is shared: composed binding needs the new
behaviour (`src/linking/composed/models.rs:904`, FR-041-AC-5), but the
*historical* checker builds the same `Catalog` (`src/checking.rs:326`), and on
that path a rational scalar is not refused anywhere downstream.
`constraints.rs:511-514` admits `<`, `<=`, `>`, `>=` for any
`NativeType::Scalar { .. }`, and `proof_type` special-cases only the integer
representation, so every other scalar falls into the
`| NativeType::Scalar { .. } => ir::ValueType::Boolean` arm
(`proof.rs:183-188`). Scenario: a `/2` model with `Ratio` at a value site, a
clause `a < b`; `validate_node` accepts the comparison and the proof builder
materializes both operands into the IR as `Boolean` — a wrong answer, not a
refusal. The change also split a pair of arms that used to agree:
`types.rs:333` now treats a bare rational scalar as equality-comparable while
`types.rs:351` still refuses a rational *field* inside a record, so the same
representation is comparable or not depending on how it is reached. Today this
is unreachable: `check_selected_profiles` (`src/linking/native.rs:19-40`) keeps
`/2` out of `link_native`, which is the only producer of a `LinkedPackage` the
checker accepts. The guard is real and tested
(`tests/native_model_profiles.rs:626-636`). But FR-041 explicitly disclaims
rational expression typing ("It establishes neither rational expression
typing/definedness", FR-041:43-45) and FR-040/TC-119 is unimplemented, so this
is admission the spec has not yet defined. Splitting the catalog by consumer, or
threading `NativeModelProfile` into `Catalog::new`, restores the refusal for the
historical path without costing composed binding anything.

**FND-002.** The change introduces `NativeModelProfile` as the typed notion of
model profile, but the string spelling now lives in four unlinked places.
`as_str` (`native_model.rs:24-29`) and `TryFrom` (`native_model.rs:37-46`) each
carry the `native-state-model/*` literals independently.
`LinkedPackage::binding_profile` (`linking.rs:199-204`) derives
`"native-state-model/1"` from `BindingProfile::Native` alone and never consults
`NativeModel::profile()`. `checking/bindings.rs:17` then gates the whole
historical checker with `if linked.binding_profile() != "native-state-model/1"`.
So the label that says which profile was linked and the check that enforces it
are in different modules, joined only by a string literal, and the actual
refusal happens in a third (`native.rs:19`). Scenario: any future `link_*`
entry point that builds `LinkedPackage { profile: BindingProfile::Native, .. }`
without calling `check_selected_profiles` gets a package that truthfully reports
`native-state-model/1` while holding `/2` models, and `bindings.rs:17` waves it
through into FND-001's path. Separately, the two halves of this feature get
different drift protection: `native-rule-model/2` was added to the shared
catalog (`wire_format.rs:35`) and is pinned by the complete-catalog assertion at
`tests/compile_command.rs:247-252`, while `native-state-model/2` is in no
catalog and has no equivalent test. `WireFormat` already demonstrates the shape
this wants — one macro-generated closed enum with `ALL`, `as_str`, `Display` and
`Serialize` (`wire_format.rs:6-52`).

**FND-003.** `/1` refuses the rational variant by decoding through `ScalarV1`
(`wire.rs:30-45`), a hand-written copy of `Scalar`'s `Integer` and `Text`
variants plus a `From` conversion (`wire.rs:65-90`). Both carry
`deny_unknown_fields`, which is right, and makes the duplication load-bearing:
the two enums must stay field-for-field identical or the two authoring profiles
fork. Nothing checks that. The only document read through *both* `FORMAT` and
`FORMAT_V2` is `tests/fixtures/native-package/model-source.json`
(`tests/native_model_profiles.rs:113-135`), whose sole scalar is
`{"name":"Id","kind":"text","max_scalars":8}` — so `ScalarV1::Integer`'s three
fields are never compared across profiles. Scenario: a later change adds
`#[serde(default)] overflow: Option<String>` to `Scalar::Integer` and misses
`ScalarV1::Integer`; a document carrying it is accepted by `/2` and rejected by
`/1` with an unknown-field error, the profiles silently diverge, and the whole
suite stays green. A one-line table test reading the same integer-bearing
fixture through both formats and asserting identical `roles`/`environment`
closes it; `#[serde(flatten)]` on a shared struct would close it structurally.

**FND-004.** The profile decision is expressed two ways in the same file.
`check_type` matches exhaustively — `ir::ValueType::Rational { .. } => match
profile { V1 => return Err(...), V2 => {} }` (`admission.rs:165-174`) — while
`preflight` uses `matches!(profile, NativeModelProfile::V1) && matches!(role.kind,
ScalarKind::Rational { .. })` (`admission.rs:66-68`). Adding a
`NativeModelProfile::V3` that must not admit rationals fails to compile at the
first and silently admits a `ScalarKind::Rational` *role* at the second. The
same pattern let the new `ScalarKind` variant through three sites without a
decision: `NativeType::integer()` (`types.rs:122-130`, `_ => None`),
`dimensionless()` (`types.rs:132-144`, `matches!`), and the text-literal check
at `constraints.rs:498`. Those three happen to be correct — a rational is not
an integer and is not dimensionless-integer — but they were never reviewed,
because nothing asked. Contrast the two sites that *were* forced to decide and
did so deliberately: `features.rs:71`/`:112` and
`runtime/validation/values.rs:113`. (`#[non_exhaustive]` on
`NativeModelProfile` at `native_model.rs:14` is consistent with `WireFormat`
and is not itself a finding.)

**FND-005.** `check_selected_profiles` iterates
`unit.imports().iter().zip(models)` (`native.rs:20`). `zip` stops at the shorter
side, so the guarantee that every import is checked rests on `select_models`
(`linking.rs:384-449`, a different module 80 lines away) pushing exactly one
model per import or returning `Err`. It does today, and the ordering is
positional and undocumented. Since this is the boundary FND-001 and FND-002 both
lean on, it deserves a comment at minimum, or an explicit length assertion.
The function's own `ok_or_else` arm (`native.rs:22-28`, "historical native
import has no admitted native model") is structurally unreachable — `native:
None` is set only by `formal_catalog` (`linking.rs:378`), never by
`native::catalog` — and has no test.

**FND-006.** The extraction is otherwise faithful, but `parents` became
`pub(super)` (`variables.rs:8`) while `ranks` and `known` stayed private, purely
so `constraints.rs:444` can read `self.variables.parents.len()`. Exposing one
raw array of a union-find to publish a count invites a caller to write to it; a
`pub(super) fn len(&self) -> usize` gives the same thing without the seam. The
carried-over `Result<bool, ()>` and `Result<(), ()>` signatures (`variables.rs:39`,
`:55`) were tolerable as private helpers inside `constraints.rs` and are less so
now that they are a module's surface — `rust-style` asks for causes, not `()`.
The header, "Shared contextual native-type variables" (`variables.rs:2`), names
a sharing that has exactly one consumer; it also omits the owning `FR-` id that
all five sibling `checking/` modules carry (`bindings.rs:2`, `constraints.rs:2`,
`inputs.rs:2`, `proof.rs:2`, `types.rs:2`) — the same omission recorded as
SR-319 FND-014.

**FND-007.** `every_rational_site_requires_one_consistent_role`
(`tests/native_model_profiles.rs:435-502`) runs twelve mutations and asserts
`Code::InvalidModelBinding` for all twelve, with the message `"mutation
{mutation}"`. FR-041-AC-2 names six distinct causes, and admission produces four
distinct diagnostics for them (`admission.rs:378`, `:389`, `:412`, `:428`).
Scenario: mutation 4 points a site at an absent value; if the representation
check at `:412` stopped firing, the unmapped-site sweep at `:428` would refuse
the same input with the same code and the test would stay green — and the
failure message would not say which mutation, only its index. The diagnostics
are reachable from the test (`error.cause` is a public `ModelSourceCause::Admission`
and `Diagnostic`'s fields are read directly at line 182), so asserting the
message alongside the code costs one line per case.

**FND-008.** `NativeModelProfile::try_from` and `UnknownNativeModelProfile`
(`native_model.rs:32-46`) have exactly one caller in the tree: the assertion at
`tests/native_model_profiles.rs:149-154`. Nothing in the crate parses a profile
spelling back out of an artifact, and the source frontend refuses unknown
*formats* through its own `ModelSourceCause::UnknownFormat` path
(`model_source.rs:256-260`). The crate's convention for string-to-identity is
`FromStr` — `ByteDigest` (`digest.rs:38`) and `ProjectionTarget`
(`lowering/target.rs:52`) — and `import.digest.value.parse()` at
`linking.rs:390` shows it in use, so `TryFrom<&str>` here means
`"native-state-model/2".parse()` does not compile while every neighbouring
identity does. Separately, FR-041:95 states the profile "is fixed by admission
and cannot be retagged afterward"; `ModelDraft` backs this with a private
`profile` field and a getter (`model_source.rs:205`, `:220-222`) while leaving
`source`, `environment` and `roles` `pub`. The test at
`tests/native_model_profiles.rs:174-181` moves exactly that payload into the
`/1` constructor — which is correct and explicit, but it means the invariant is
enforced by the absence of a setter, not by ownership.

**FND-009.** `model_features` and `native_type` refuse a rational role with
`ScalarKind::Rational { .. } => return Err(invalid_model())` written *inside* a
`match` that is an argument to `features.insert(...)` (`features.rs:69-74`,
`:110-115`). `native_type` gained a `Result` return and three `?` propagations
solely for that arm. `features::derive` consumes a `CheckedPackage`, which only
`checking::check` produces, which only accepts `link_native` output, which
refuses `/2` — so no supported input reaches either arm and neither has a test.
Defensible defence in depth, but it should be labelled as such, and a `return`
inside an expression argument is a reading hazard worth hoisting into a
statement.

## What holds up

- **`/1` is genuinely frozen.** `explicit_profiles_preserve_the_frozen_historical_artifact`
  compares the `/1` artifact byte-for-byte against the independent package
  fixture, then asserts the `/2` artifact differs from it in exactly the
  `profile` member and nowhere else
  (`tests/native_model_profiles.rs:124-143`) — a far stronger check than "the
  digests differ". `/1` refuses a rational declaration, an unmapped rational
  declaration, and a rational *role* on an otherwise legal legacy model
  (`:169-206`).
- **Bounds are the IR's, unrepaired.** `lower_scalar` calls the real
  `ir::RationalType::new` (`lower.rs:133`) and only attaches the original span
  to the upstream diagnostic. I checked the pinned constructor
  (`quire-contract-ir` 690bde7, `src/expression.rs:110-132`): it validates
  `minimum <= maximum` and `1 <= maximum_denominator <= i64::MAX` and performs
  no normalization, matching FR-041:82-84 exactly. `i64::MIN`/`i64::MAX`, both
  sides of 2^53, and denominators 1 and `i64::MAX` round-trip to literal
  expectations (`tests/native_model_profiles.rs:211-228`), and the tests
  correctly separate strict-decoder refusals from
  `ir::DiagnosticCode::InvalidNumericBounds` (`:311-352`).
- **Charge-before-work and arithmetic.** `spend` uses `checked_sub`
  (`admission.rs:38`), `StringBudget::text` uses `checked_sub`
  (`artifact.rs:21`), `BoundedBytes::write` uses `checked_add`
  (`artifact.rs:233-236`). `check_type` charges the node *before* inspecting the
  variant (`admission.rs:152`), so a rational node costs the same as any other.
  No bare `as` cast was added; `u64::try_from` guards the canonicalization limit
  (`artifact.rs:256`). The limit test derives its artifact size from an
  independently written JSON oracle rather than from the producer's own output,
  and says so (`tests/native_model_profiles.rs:976-984`).
- **The artifact really carries the bounds.** `CanonicalProfile::V1`
  canonicalization serializes `RationalType`'s three fields through the derived
  `Serialize` (IR `canonical.rs:421-457`, `expression.rs:103-108`), so the
  mutation sensitivity asserted at `tests/native_model_profiles.rs:567-577` is
  structural, not incidental. Set-like reordering of roles and sites is
  normalized (`native_model.rs:296-310`) and asserted byte-identical (`:553-564`).
- **Composed binding is exercised for real.** `resolve_type`, `field`, digest
  substitution (including the IR canonical and source digests), foreign package
  and missing export all run through the public `bind_models` boundary
  (`:690-793`), and same-representation distinct owners are separated by nominal
  identity, not by bytes (`:853-910`).
- **Tests are public-API only and fully tagged.** All 14 carry
  `#[trace("TC-120", "FR-041-AC-N")]`; `quire coverage` binds all seven ACs and
  TC-120. None is `#[ignore]`d, `#[should_panic]`, assertion-free, or
  clock-dependent. No `TODO`/`FIXME`/`dbg!`/`unwrap`/`expect`/`panic!`/
  `#[allow]` was added to `src/`; no `unsafe`, no `#[cfg(test)]` branch in
  production code, no suppressed warning, no dependency or workflow change.

## Not findings

- `role.sites[0]` in composed `scalar_type` (`models.rs:877`) is still guarded
  by admission rejecting empty `sites` (`admission.rs:374`), and rational does
  not weaken that.
- `check_scalars` is not profile-aware (`admission.rs:366-433`). That is correct
  layering, not a hole: `preflight` has already refused both a rational role and
  a rational declaration under `/1` before the catalog is built
  (`admission.rs:24`, `:66`, `:165`). A one-line comment saying so would help.
- Running `check_selected_profiles` *after* `native::catalog` and `select_models`
  (`linking.rs:309-311`) means a `/2` model is canonicalized before being
  refused. That is required — selection is what decides which models matter —
  and the limits are still enforced first, so no budget escapes.
- The `check_string_content` sweep does not charge the rational bound digits
  (`artifact.rs:90-111`). It is documented as a lower bound
  (`admission.rs:131-133`) and the exact ceiling is `BoundedBytes`.
- `FR-040-AC-4`'s "rational normalization-before-bounds" is not implemented here
  and should not be: the pinned IR does it at the literal-checking boundary
  (`expression.rs:1682-1692`), not in the type constructor, so FR-041's
  producer-only scope is the right cut.

## Correction disposition (a4344d0)

Targeted re-review of correction 96d9240 and its integration merge a4344d0,
same skills and gates. Binding indexes inherited from PR48 have their own
review and were not re-reviewed here. Historical proof semantics are unchanged
and no false nominal equivalence was introduced: `Catalog::historical` restores
the exact pre-FR-041 `Rational => None` refusal (`types.rs:346`), `equality`
refuses a rational scalar reached directly *or* through a record field
(`types.rs:386`, `:403`), the `integer()`/`dimensionless()` tightening is
redundant with admission's role↔representation check, and the remaining
rewritten matches are behaviour-preserving.

**Verdict: CONDITIONAL cleared.** Seven of nine findings closed, two partially;
the residue is low and latent. All five gates re-run and pass.

| ID | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | fixed | private `Interpretation` + `Catalog::historical`/`composed` (`types.rs:205-225`, `:346`), wired at `checking.rs:326` / `linking/composed/models.rs:179`; control `types.rs:438` |
| FND-002 | fixed | `binding_profile` derives from `NativeModelProfile::V1.as_str()` (`linking.rs:200`); `require_historical_native` rechecks actual selections (`linking.rs:207`, `checking/bindings.rs:17`) |
| FND-003 | partial | cross-profile parity test on one fixture's integer/text meanings and artifact JSON (`tests/native_model_profiles.rs:171`); `ScalarV1` remains a hand copy (`model_source/wire.rs:32`) |
| FND-004 | fixed | exhaustive profile×kind decisions at `admission.rs:66`, `:173`, `:400`, `:485`; `types.rs:110`, `:139`, `:385`; `constraints.rs:500`; `features.rs:69`, `:114` |
| FND-005 | fixed | explicit import/model length refusal and ordering comment (`linking/native.rs:23`, `linking.rs:397`); the former unreachable arm now covered (`linking.rs:1060`) |
| FND-006 | fixed | `parents` private, `len()` accessor, typed `TypeConflict`, `FR-016/040` header (`checking/variables.rs:2-30`) |
| FND-007 | partial | per-mutation descriptions and an exact foreign-declaration-locus assertion (`tests/native_model_profiles.rs:512`); the six FR-041-AC-2 causes still share one asserted code |
| FND-008 | fixed | `FromStr` added with `TryFrom` delegating (`native_model.rs:45`); retag control `tests/native_model_profiles.rs:217` |
| FND-009 | fixed | arms hoisted to statements with defensive comments (`features.rs:76`, `:121`) |

### Gates (a4344d0)

Whole sequential batch under one `flock /tmp/quire-heavy-check.lock`, with
`CARGO_BUILD_JOBS=1`, `CARGO_TARGET_DIR=/tmp/formalization-a-language-target`,
`nice -n 10`, `--locked`; `src/lib.rs` mtime touched once under the lock
(content unchanged) to defeat cross-worktree reuse. Exit statuses captured
directly, no pipes.

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (0) |
| `clippy --all-targets --no-default-features -- -D warnings` | pass (0) |
| `clippy --all-targets --all-features -- -D warnings` | pass (0) |
| `test --locked --no-default-features -- --test-threads=1` | pass (0) — 440 passed, 0 failed, 4 ignored |
| `test --locked --all-features -- --test-threads=1` | pass (0) — 456 passed, 0 failed, 4 ignored |
| `cargo deny check` | not applicable — no `deny.toml` |

`tests/native_model_profiles.rs` runs 16 tests in both configurations. The 4
ignored lanes are the pre-existing named IT-004/LC04 ones.

### Residual findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | Structural half of FND-003 open: the parity test uses one fixed fixture, so a field added to `Scalar::Integer` but not `ScalarV1::Integer` still forks the two profiles with the suite green | src/model_source/wire.rs:32, tests/native_model_profiles.rs:171 |
| FND-011 | low | `FromStr` matches over a hand-maintained `[Self::V1, Self::V2]`; a future variant fails to parse instead of failing to compile, and neither `TryFrom` nor `FromStr` has a production caller (SR-324 FND-006) | src/native_model.rs:48 |
| FND-012 | low | `proof_type`'s `NativeType::Scalar { .. }` catch-all still maps any non-integer scalar to `ir::ValueType::Boolean` — the one FND-004 site left absorbing a new representation silently; unreachable for rational today | src/checking/proof.rs:184 |
| FND-013 | low | FND-007 residue: twelve mutations still assert one shared code, so a representation check that stopped firing would be masked by the unmapped-site sweep | tests/native_model_profiles.rs:607 |
| FND-014 | low (style) | `bindings.rs` rewrites `error.phase` on a diagnostic returned by the linker rather than the producer taking the phase | src/checking/bindings.rs:17 |

The two new in-`src` `#[cfg(test)]` controls (`linking.rs:1006`,
`types.rs:438`) deliberately bypass `link_native`/`Catalog::new` to reach
defensive arms unreachable from the public API; both are commented as such, run
the real `check`/`admit` paths, and add no production test seam. That is the
correct trade-off here, and it is the only departure from this branch's
otherwise public-API-only test posture.
