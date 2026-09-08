# Dependency and generated-artifact inventory

LC01 uses pinned Logos 0.16.1 for token recognition and serde_json 1.0.151
for JSON strings/CLI output. Cargo.lock fixes the transitive resolution below.
This records package metadata, not a public distribution notice bundle.
Existing dependency grants are preserved. Release remains a separate review.

| Package | Version | Declared grant |
|---|---|---|
| aho-corasick | 1.1.5 | `Unlicense OR MIT` |
| fnv | 1.0.7 | `Apache-2.0 / MIT` |
| itoa | 1.0.18 | `MIT OR Apache-2.0` |
| logos | 0.16.1 | `MIT OR Apache-2.0` |
| logos-codegen | 0.16.1 | `MIT OR Apache-2.0` |
| logos-derive | 0.16.1 | `MIT OR Apache-2.0` |
| memchr | 2.8.3 | `Unlicense OR MIT` |
| proc-macro2 | 1.0.107 | `MIT OR Apache-2.0` |
| quote | 1.0.47 | `MIT OR Apache-2.0` |
| regex-automata | 0.4.18 | `MIT OR Apache-2.0` |
| regex-syntax | 0.8.11 | `MIT OR Apache-2.0` |
| serde | 1.0.229 | `MIT OR Apache-2.0` |
| serde_core | 1.0.229 | `MIT OR Apache-2.0` |
| serde_derive | 1.0.229 | `MIT OR Apache-2.0` |
| serde_json | 1.0.151 | `MIT OR Apache-2.0` |
| syn | 2.0.119 | `MIT OR Apache-2.0` |
| syn | 3.0.5 | `MIT OR Apache-2.0` |
| unicode-ident | 1.0.24 | `(MIT OR Apache-2.0) AND Unicode-3.0` |
| zmij | 1.0.23 | `MIT` |

All resolved packages offer MIT, with unicode-ident additionally requiring its
Unicode-3.0 grant. The legacy `Apache-2.0 / MIT` fnv spelling is retained above
as declared by its manifest. Original notice/license files stay in dependency
packages. No existing project code or external parser corpus is copied.
New test fixtures and the declarative token specification are authored here
under AGPL-3.0-only. The Logos derive macro generates recognizer code at build
time; generated build files are not checked in. A shipped binary/package needs
its own included-artifact and notices inventory before publication.
