---
id: TC-282
title: "Every resolve_libraries refusal classifies to an I2 rule, a §4 condition, E3 resolution, or a named exception"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-282: Every resolve_libraries refusal classifies to an I2 rule, a §4 condition, E3 resolution, or a named exception

## Description

Verify the complete, honest classification FR-087's owner ruling on
QSL-158 (2026-09-21) states (Description, item 3, owner ruling (e)): every
`LibraryRefusal` variant `resolve_libraries` (relocated into `library`) can
return classifies to exactly one of: an ADR-011 I2 "Package import graph
rule" (`:203-210`), the §4 binding's condition 2 (digest/preimage
admission, `verify_package`), the §4 binding's condition 3 (identity
pinned by the library lock, `:513`), or E3 name resolution — except
`DuplicatePackageId`, the one stated exception, which lies outside all
four. Scope: FR-087-AC-12.

## Test Procedure

1. Enumerate every variant `LibraryRefusal` defines today: `PackageIdMismatch`,
   `InvalidPreimage`, `UndeclaredExport`, `InvalidQualifier`,
   `ConflictingDefinition`, `ImportCycle`, `StaleDependency` (both of its
   `StaleCause` values, `RevisionMismatch` and `ByteDigestMismatch`, taken
   separately since they arise from different conditions), `MissingImport`,
   and `DuplicatePackageId` — nine variants (ten counting `StaleDependency`'s
   two causes separately), plus any variant its refusal type gains after
   relocation.
2. For each variant, read the code path that raises it and classify it
   against this fixed table (not the earlier, incomplete "one for one with
   an I2 rule" claim):

   | Variant | Classification | Why (read from the code) |
   | --- | --- | --- |
   | `PackageIdMismatch` | §4 condition 2 | Raised inside `verify_package`'s own digest recomputation and comparison — condition 2 itself. |
   | `InvalidPreimage` | Per-package admission within `verify_package` | Raised after condition 2's digest comparison already completed (`identity_preimage` matched its declared `package_id`); validates the preimage's own structure. Grouped with condition 2 only because the same reused `verify_package` call raises it, not because condition 2 depends on it. |
   | `UndeclaredExport` | Per-package admission within `verify_package` | Raised by the same `verify_package` call, after the preimage structure check, comparing the package's own declared exports against its own projected declarations. Per-package admission, not an import-graph or name question, and not part of condition 2's digest check either. |
   | `InvalidQualifier` | E3 name resolution | Validates an `as` qualifier used only for binding `a::Name` references; moves conceptually with `resolve_name` to E3. |
   | `ConflictingDefinition` | I2 rule 2 | Two import paths claim one library identity with different selections — I2's second rule exactly. |
   | `ImportCycle` | I2 rule 3 | An import-graph cycle — I2's third rule exactly. |
   | `StaleDependency{RevisionMismatch}` | §4 condition 3 | The supplied package's `package_id` and library identity already match the import; only the lock-recorded version disagrees — the identity-pinned-by-the-lock question, not a graph-structure question. |
   | `StaleDependency{ByteDigestMismatch}` | I2 rule 1 | The import's `package_id` is not present among the supplied packages under its identity — "not listed," I2's first rule. |
   | `MissingImport` | I2 rule 1 | No supplied package matches the import's identity at all — "an import is missing," I2's first rule. |
   | `DuplicatePackageId` | Named exception (none of the four) | Raised only after both supplied packages have already, independently, passed `verify_package`'s digest check — so each one's `package_id` already equals the digest recomputed over its own `identity_preimage`. Two packages sharing one `package_id` therefore share byte-identical `identity_preimage` bytes; they can only still differ in the `LibraryPackage` fields the preimage excludes (`version`, `imports`, `exports`) — conflicting metadata over identical identity content, not two different contents colliding on one digest. This is the reverse of `ConflictingDefinition` (one identity, two competing selections from different import sites) and needs no ADR-011 `:203-210` I2 rule of its own; it is a content-addressing precondition the `by_id` index needs, independent of I2, the §4 binding, and E3. |

3. Confirm every variant in step 1 receives exactly the classification step
   2's table states; a variant classified to a different bucket than the
   table (for example `StaleDependency{RevisionMismatch}` mapped to I2 rule
   1 instead of condition 3, or `DuplicatePackageId` mapped to I2 rule 2
   instead of the named exception), or left unclassified, fails this step.
4. Construct one concrete fixture per row in step 2's table (ten
   fixtures, one per row, each varying exactly the one fault its row
   names: a package whose declared `package_id` does not equal the digest
   recomputed over its own `identity_preimage`, all else valid, for
   `PackageIdMismatch` — `resolve_libraries` has no schema-version check to
   combine this with, so the fixture is a digest mismatch only, not a
   second, unrelated fault; a structurally malformed preimage for
   `InvalidPreimage`; a package whose declared export matches no
   declaration for `UndeclaredExport`; a malformed `as` qualifier for
   `InvalidQualifier`; a diamond import with two conflicting selections for
   `ConflictingDefinition`; a two-package import cycle for `ImportCycle`; a
   supplied package matching identity and `package_id` but not `version`
   for `StaleDependency{RevisionMismatch}`; a supplied package whose
   `package_id` does not match the import's for
   `StaleDependency{ByteDigestMismatch}`; no supplied package under the
   imported identity at all for `MissingImport`; and two supplied packages
   sharing one byte-identical `identity_preimage` (and hence one recomputed
   `package_id`, each independently passing its own digest check) but
   differing in `version`, `imports`, or `exports` for `DuplicatePackageId`)
   and
   confirm `resolve_libraries` refuses each with exactly the variant its
   row names.
5. Confirm this criterion does not require a new refusal variant:
   `resolve_libraries`' existing nine-variant refusal set, unchanged in
   shape by this requirement (Description, item 3, owner ruling (a)), is
   sufficient to cover every classification, including the
   `DuplicatePackageId` exception.

## Expected Results

- Steps 1-3: every `LibraryRefusal` variant is classified exactly as step
  2's table states; an unclassified variant, or one classified to the
  wrong bucket (including forcing `DuplicatePackageId` into an I2 rule, or
  `StaleDependency{RevisionMismatch}` into I2 rule 1), fails this step and
  names the variant.
- Step 4: each of the ten fixtures (nine variants, `StaleDependency` split
  by cause) refuses with exactly its table-predicted variant; a different
  variant, or an admission, fails this step.
- Step 5: no new refusal variant is required; a passing implementation
  needs no change to `LibraryRefusal`'s variant set beyond relocation.
