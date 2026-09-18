---
id: TC-149
title: "Refuse a malformed or under-specified vendoring pin"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/NFR-011, type: verifies }
---
# TC-149: Refuse a malformed or under-specified vendoring pin

## Description

Verify that `xtask`'s manifest loading, `revendor` and `revendor-check`
refuse a pin or invocation that does not fully and safely specify what to
vendor, before any byte is read or written. Scope: NFR-011-AC-1.

## Test Procedure

1. Load a `VENDOR.json` whose commit is short or non-hexadecimal, whose two
   sources vendor the same destination path, whose `schema_version` is
   unsupported, or which carries an unknown field.
2. Load a `VENDOR.json` whose pinned-file `path`, `dest_prefix` or external
   `dest` is absolute, contains a `\`, or contains a `.`/`..` component.
3. Call `revendor` on a manifest with a `qspec`-kind source and no
   `--qspec-clone`/`qspec_clone` given.
4. Run `cargo xtask revendor-check --qspec-clone <path>` and inspect the
   `USAGE` text `cargo xtask revendor-check` documents.

## Expected Results

- Step 1 refuses each case with `Error::InvalidManifest` (or, for the unknown
  field, `Error::Manifest`), naming the manifest path and the offending
  value; no path is read.
- Step 2 refuses each case with `Error::InvalidManifest` naming the manifest
  path and the unsafe path text.
- Step 3 refuses with `Error::MissingClone`; no network fetch is attempted.
- Step 4 refuses with `Error::CheckRefusesQspecClone`, and the usage text for
  `revendor-check` never advertises `--qspec-clone`.
