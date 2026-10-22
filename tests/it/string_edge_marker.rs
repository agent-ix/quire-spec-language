// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-162 steps 15 and 16 (FR-064-AC-14, FR-064-AC-15): the `#[string_edge]`
//! marker is `ix-trace-rs`'s. The workspace defines no attribute macro, no
//! re-export or alias of the marker and no marker-only crate, and every
//! package whose source carries the marker declares `ix-trace-rs` as a normal
//! dependency. `cargo check` of each package then resolves each marker to
//! `ix_trace_rs::string_edge`.

use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

struct Member {
    name: String,
    normal: Vec<String>,
    proc_macro: bool,
    target_roots: Vec<PathBuf>,
}

fn members() -> Vec<Member> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
    metadata["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .map(|package| Member {
            name: package["name"].as_str().expect("a name").to_owned(),
            normal: package["dependencies"]
                .as_array()
                .expect("a dependency list")
                .iter()
                .filter(|dependency| dependency["kind"].is_null())
                .map(|dependency| dependency["name"].as_str().expect("a name").to_owned())
                .collect(),
            target_roots: package["targets"]
                .as_array()
                .expect("a target list")
                .iter()
                .filter(|target| {
                    target["kind"].as_array().is_some_and(|kinds| {
                        kinds.iter().any(|kind| {
                            ["lib", "bin", "proc-macro", "custom-build"]
                                .contains(&kind.as_str().unwrap_or_default())
                        })
                    })
                })
                .map(|target| PathBuf::from(target["src_path"].as_str().expect("a source path")))
                .collect(),
            proc_macro: package["targets"]
                .as_array()
                .expect("a target list")
                .iter()
                .any(|target| {
                    target["kind"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind == "proc-macro"))
                }),
        })
        .collect()
}

/// Every `.rs` file under the directory of each production target (library,
/// binary, proc-macro, build script) of the package, with its text. A target
/// whose root is `lib.rs` beside `Cargo.toml` (`tools/arch-lint`) is covered
/// the same way as one under `src/`.
fn sources(member: &Member) -> Vec<(PathBuf, String)> {
    fn walk(directory: &Path, found: &mut Vec<(PathBuf, String)>) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|name| name != "target" && name != "tests")
                {
                    walk(&path, found);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && !found.iter().any(|(seen, _)| *seen == path)
            {
                let text = fs::read_to_string(&path).expect("a readable source file");
                found.push((path, text));
            }
        }
    }
    let mut found = Vec::new();
    for root in &member.target_roots {
        walk(root.parent().expect("a target directory"), &mut found);
    }
    found
}

fn carries_marker(text: &str) -> bool {
    text.contains("#[string_edge]")
        || text.contains("ix_trace_rs::string_edge]")
        || text.contains("use ix_trace_rs::string_edge;")
}

/// What a source file defines, re-exports or aliases that FR-064-AC-14 rules
/// out: an attribute macro (or any `#[proc_macro_attribute]` function) named
/// `string_edge`; a `use` that renames `string_edge`, or that makes it or the
/// `ix_trace_rs` glob visible beyond the module; and any `qsl_attrs` path.
fn marker_violations(source: &str) -> Vec<String> {
    use syn::visit::Visit;

    #[derive(Default)]
    struct Finder {
        found: Vec<String>,
    }

    impl Finder {
        fn tree(&mut self, tree: &syn::UseTree, public: bool, in_marker_crate: bool) {
            match tree {
                syn::UseTree::Path(path) => {
                    let marker_crate = path.ident == "ix_trace_rs";
                    if path.ident == "qsl_attrs" {
                        self.found.push("a `qsl_attrs` path".to_owned());
                    }
                    self.tree(&path.tree, public, marker_crate);
                }
                syn::UseTree::Name(name) if name.ident == "string_edge" && public => {
                    self.found.push("a re-export of `string_edge`".to_owned());
                }
                syn::UseTree::Rename(rename) if rename.ident == "string_edge" => {
                    self.found.push("an alias of `string_edge`".to_owned());
                }
                syn::UseTree::Glob(_) if public && in_marker_crate => {
                    self.found
                        .push("a re-export of `ix_trace_rs::*`".to_owned());
                }
                syn::UseTree::Group(group) => {
                    for item in &group.items {
                        self.tree(item, public, in_marker_crate);
                    }
                }
                _ => {}
            }
        }
    }

    impl<'ast> Visit<'ast> for Finder {
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            let public = !matches!(item.vis, syn::Visibility::Inherited);
            self.tree(&item.tree, public, false);
        }

        fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
            let is_attribute_macro = item
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("proc_macro_attribute"));
            if is_attribute_macro && item.sig.ident == "string_edge" {
                self.found
                    .push("an attribute macro named `string_edge`".to_owned());
            }
            syn::visit::visit_item_fn(self, item);
        }

        fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
            let first = attr.path().segments.first();
            if first.is_some_and(|segment| segment.ident == "qsl_attrs") {
                self.found.push("a `qsl_attrs` path".to_owned());
            }
            syn::visit::visit_attribute(self, attr);
        }
    }

    let file = syn::parse_file(source).expect("a parsable source file");
    let mut finder = Finder::default();
    finder.visit_file(&file);
    finder.found
}

/// The guard rejects each alias, re-export and definition form, and accepts
/// the plain import and qualified use.
#[trace("TC-162", "FR-064-AC-14")]
#[test]
fn the_guard_rejects_aliases_reexports_and_definitions() {
    for rejected in [
        "use ix_trace_rs::string_edge as edge;",
        "use ix_trace_rs::string_edge as _;",
        "pub use ix_trace_rs::string_edge;",
        "pub(crate) use ix_trace_rs::string_edge;",
        "pub use ::ix_trace_rs::string_edge;",
        "pub use ix_trace_rs::{string_edge, trace};",
        "pub use ix_trace_rs::{trace, string_edge as edge};",
        "pub use ix_trace_rs::*;",
        "mod inner { pub(super) use ix_trace_rs::string_edge; }",
        "#[proc_macro_attribute]\npub fn string_edge(a: TokenStream, i: TokenStream) -> TokenStream { i }",
        "use qsl_attrs::string_edge;",
        "#[qsl_attrs::string_edge]\nfn f() {}",
    ] {
        assert!(
            !marker_violations(rejected).is_empty(),
            "the guard accepted `{rejected}`"
        );
    }
    for accepted in [
        "use ix_trace_rs::string_edge;\n#[string_edge]\nfn f() {}",
        "use ix_trace_rs::{string_edge, trace};",
        "#[ix_trace_rs::string_edge]\nfn f() {}",
        "#[::ix_trace_rs::string_edge]\nfn f() {}",
        "use ix_trace_rs::*;",
        "#[proc_macro_attribute]\npub fn other(a: TokenStream, i: TokenStream) -> TokenStream { i }",
    ] {
        assert_eq!(
            marker_violations(accepted),
            Vec::<String>::new(),
            "the guard rejected `{accepted}`"
        );
    }
}

/// FR-064-AC-14: no attribute macro, no marker-only crate, no re-export or
/// alias of the marker anywhere in the workspace's production targets.
#[trace("TC-162", "FR-064-AC-14")]
#[test]
fn the_workspace_defines_no_string_edge_marker_of_its_own() {
    let members = members();
    assert!(
        members.iter().any(|member| member.name == "arch-lint"),
        "the guard must cover tools/arch-lint"
    );
    for member in &members {
        assert!(
            !member.proc_macro,
            "{} is a proc-macro crate; the marker is ix-trace-rs's",
            member.name
        );
        let files = sources(member);
        assert!(
            !files.is_empty(),
            "{} has no production source",
            member.name
        );
        for (path, text) in files {
            let violations = marker_violations(&text);
            assert!(
                violations.is_empty(),
                "{} holds {}",
                path.display(),
                violations.join(", ")
            );
        }
    }
}

/// FR-064-AC-15: every package that carries the marker names `ix-trace-rs`
/// among its normal dependencies.
#[trace("TC-162", "FR-064-AC-15")]
#[test]
fn every_package_that_uses_the_marker_depends_on_ix_trace_rs() {
    let mut users = 0;
    for member in members() {
        if sources(&member)
            .iter()
            .any(|(_, text)| carries_marker(text))
        {
            users += 1;
            assert!(
                member.normal.iter().any(|name| name == "ix-trace-rs"),
                "{} uses the marker without a normal ix-trace-rs dependency",
                member.name
            );
        }
    }
    assert!(users > 0, "no package carries the marker");
}
