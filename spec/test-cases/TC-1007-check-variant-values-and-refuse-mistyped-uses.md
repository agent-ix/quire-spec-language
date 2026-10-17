---
id: TC-1007
title: "A variant case literal and every variant-producing form check, and each mistyped use refuses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-360
    type: verifies
---
# TC-1007: A variant case literal and every variant-producing form check, and each mistyped use refuses

## Description

Verify check typing of variant values. Scope: FR-360-AC-4 and FR-360-AC-5.

## Test Procedure

1. Check `f(t: Config::Tier): Config::Tier { t }`, a field read, a `let`, an `Option` `value`, a collection element, a conditional of two cases, and a literal at a field, a parameter and an `Option` slot.
2. Check a clause naming `Config::Tier` and `Config::Tier::Gold`.
3. Check each mistyped use of FR-360-AC-5: another variant type's case, Integer, a bare case name, a collection or `Option` of the variant type at the wrong type, an undeclared and a letter-case-variant case, an unknown type name and an ordering operator; then the spellings `"Gold"` (checked), `none` (S2), `case t when { true } true` and a well-formed `case` over a variant.
4. Check a clause context and an operation-body target naming `Config::Tier`.
5. Admit a variant type with a case `Gold Plus` and check `Config::Tier::Gold`, then the spelling `Config::Tier::Gold Plus`.

## Expected Results

- Step 1 checks every form as `Config::Tier`.
- Step 2 raises no `unsupported_construct`.
- Step 3 refuses each use `ill_typed`/`type-mismatch` at the expression naming the expected and actual types, an undeclared case and a mistyped value `ill_typed`/`type-mismatch`, an unknown type name `missing_declaration`/`missing-name`, the ordering operators `ill_typed`/`operator-ineligible`; `"Gold"` refuses at the checker `ill_typed`/`type-mismatch` at the literal naming `String`, `none` refuses at S2 `unsupported_construct` (`UnrepresentedConstruct`, `Primary`) at the `none`, `case t when ...` refuses at S1 `invalid_syntax` (`UnexpectedToken`) at the `when` token, and a well-formed `case` over a variant refuses at the checker `ill_typed`/`type-mismatch` at the scrutinee (FR-318).
- Step 4 refuses `missing_declaration`/`missing-name` at the name.
- Step 5 admits the type, checks the writable case, and does not parse the unwritable spelling.
- Assertions compare typed codes, causes, keys and paths.
