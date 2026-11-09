// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-568 (ADR-035 CG-4): the `unsafe-confinement` check. `unsafe` code
//! lives in `qsl_jit::exec` and nowhere else in the workspace.
//!
//! The allowed module list is the one entry [`ALLOWED`]. The check reads the
//! workspace manifests (`members` of the root `Cargo.toml`) and parses every
//! `.rs` file of every member package with `syn`, so a file reached through
//! `#[path]` or `include!` is scanned as a file of its package.

use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::Visit;

use crate::error::{Error, Result};

/// The allowed `(package, module)` list: exactly one entry.
pub(crate) const ALLOWED: [(&str, &str); 1] = [("qsl-jit", "exec")];

/// One violation: package, file and item.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Finding {
    pub(crate) package: String,
    pub(crate) file: PathBuf,
    pub(crate) item: String,
}

/// Manifest text with `#` comments removed.
fn uncommented(text: &str) -> String {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The members of the workspace rooted at `root`, and the findings for a
/// manifest the check cannot read: it fails closed rather than skip a member.
fn members(root: &Path, findings: &mut Vec<Finding>) -> Result<Vec<(String, PathBuf)>> {
    let manifest = root.join("Cargo.toml");
    let text =
        uncommented(&fs::read_to_string(&manifest).map_err(|error| Error::io(&manifest, error))?);
    let mut directories = Vec::new();
    if text.lines().any(|line| line.trim() == "[package]") {
        directories.push(PathBuf::from("."));
    }
    let mut in_workspace = false;
    let mut collecting = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_workspace = line == "[workspace]";
            collecting = false;
            continue;
        }
        if in_workspace && line.starts_with("members") {
            collecting = true;
        }
        if collecting {
            for piece in line.split('"').skip(1).step_by(2) {
                if piece.contains('*') || piece.contains('?') {
                    findings.push(Finding {
                        package: "workspace".to_owned(),
                        file: PathBuf::from("Cargo.toml"),
                        item: format!("member glob `{piece}` is not supported"),
                    });
                } else if piece != "." {
                    directories.push(PathBuf::from(piece));
                }
            }
            if line.contains(']') {
                collecting = false;
            }
        }
    }
    let mut found = Vec::new();
    for directory in directories {
        let path = root.join(&directory).join("Cargo.toml");
        let Ok(text) = fs::read_to_string(&path) else {
            findings.push(Finding {
                package: directory.display().to_string(),
                file: directory.join("Cargo.toml"),
                item: "member manifest is missing or unreadable".to_owned(),
            });
            continue;
        };
        let text = uncommented(&text);
        let name = text
            .lines()
            .skip_while(|line| line.trim() != "[package]")
            .find_map(|line| {
                let (key, value) = line.split_once('=')?;
                (key.trim() == "name").then(|| value.trim().trim_matches('"').to_owned())
            })
            .unwrap_or_else(|| directory.display().to_string());
        found.push((name, root.join(directory)));
    }
    Ok(found)
}

/// Whether the workspace root declares `unsafe_code = "forbid"` in
/// `[workspace.lints.rust]`, which `[lints] workspace = true` inherits.
fn workspace_forbids(root: &Path) -> bool {
    let Ok(text) = fs::read_to_string(root.join("Cargo.toml")) else {
        return false;
    };
    let mut in_section = false;
    for line in uncommented(&text).lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == "[workspace.lints.rust]";
        } else if in_section {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "unsafe_code" && value.trim().trim_matches('"') == "forbid" {
                    return true;
                }
            }
        }
    }
    false
}

/// The `unsafe_code` level a manifest declares in `[lints.rust]`, and whether
/// it inherits the workspace's lints.
fn manifest_level(text: &str) -> (Option<String>, bool) {
    let mut section = String::new();
    let (mut level, mut inherits) = (None, false);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line.to_owned();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim().trim_matches('"'));
        if section == "[lints.rust]" && key == "unsafe_code" {
            level = Some(value.to_owned());
        }
        if section == "[lints]" && key == "workspace" && value == "true" {
            inherits = true;
        }
    }
    (level, inherits)
}

/// Crate roots of a package: its `src/lib.rs`, `src/main.rs` and any
/// `path = "..."` the manifest names.
fn crate_roots(directory: &Path, manifest: &str) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = ["src/lib.rs", "src/main.rs"]
        .iter()
        .map(|relative| directory.join(relative))
        .filter(|path| path.is_file())
        .collect();
    for line in manifest.lines() {
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "path" {
                let path = directory.join(value.trim().trim_matches('"'));
                if path.extension().is_some_and(|ext| ext == "rs") && path.is_file() {
                    roots.push(path);
                }
            }
        }
    }
    roots
}

fn rust_files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            // A nested package is scanned as its own member.
            let nested = path.join("Cargo.toml").is_file();
            if name != "target" && name != ".git" && !nested {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

struct Scan {
    items: Vec<String>,
    lowering: Vec<(String, bool)>,
}

fn lowers_unsafe(attribute: &syn::Attribute) -> bool {
    let name = attribute
        .path()
        .get_ident()
        .map(ToString::to_string)
        .unwrap_or_default();
    matches!(name.as_str(), "allow" | "expect" | "warn")
        && attribute
            .meta
            .require_list()
            .is_ok_and(|list| list.tokens.to_string().contains("unsafe_code"))
}

impl Scan {
    fn attribute(&mut self, attribute: &syn::Attribute, on_mod_exec: bool) {
        if lowers_unsafe(attribute) {
            self.lowering.push((
                format!(
                    "#[{}(unsafe_code)]",
                    attribute
                        .path()
                        .get_ident()
                        .map_or_else(String::new, ToString::to_string)
                ),
                on_mod_exec,
            ));
        }
    }
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        self.attribute(attribute, false);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if item.sig.unsafety.is_some() {
            self.items.push(format!("unsafe fn {}", item.sig.ident));
        }
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if item.sig.unsafety.is_some() {
            self.items.push(format!("unsafe fn {}", item.sig.ident));
        }
        syn::visit::visit_impl_item_fn(self, item);
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if item.unsafety.is_some() {
            self.items.push("unsafe impl".to_owned());
        }
        syn::visit::visit_item_impl(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if item.unsafety.is_some() {
            self.items.push(format!("unsafe trait {}", item.ident));
        }
        syn::visit::visit_item_trait(self, item);
    }

    fn visit_expr_unsafe(&mut self, expression: &'ast syn::ExprUnsafe) {
        self.items.push("unsafe block".to_owned());
        syn::visit::visit_expr_unsafe(self, expression);
    }

    /// The outer attributes of `mod exec` are the one place a lowering
    /// attribute is allowed (the caller decides by file); every attribute
    /// inside the module, and on any other item, is visited as usual.
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        for attribute in &item.attrs {
            self.attribute(attribute, item.ident == "exec");
        }
        if let Some((_, items)) = &item.content {
            for inner in items {
                self.visit_item(inner);
            }
        }
    }
}

fn crate_root_forbids(root: &Path) -> bool {
    fs::read_to_string(root).is_ok_and(|text| {
        text.lines()
            .any(|line| line.trim().replace(' ', "") == "#![forbid(unsafe_code)]")
    })
}

/// Run the check over the workspace at `root`.
pub(crate) fn check(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let workspace_lints_forbid = workspace_forbids(root);
    for (package, directory) in members(root, &mut findings)? {
        let manifest_path = directory.join("Cargo.toml");
        let manifest =
            fs::read_to_string(&manifest_path).map_err(|error| Error::io(&manifest_path, error))?;
        let (level, inherits) = manifest_level(&manifest);
        let is_jit = ALLOWED.iter().any(|(name, _)| *name == package);
        let mut finding = |file: &Path, item: String| {
            findings.push(Finding {
                package: package.clone(),
                file: file.strip_prefix(root).unwrap_or(file).to_path_buf(),
                item,
            });
        };
        if is_jit {
            if level.as_deref() != Some("deny") {
                finding(&manifest_path, "unsafe_code is not `deny`".to_owned());
            }
        } else {
            let roots = crate_roots(&directory, &manifest);
            let forbidden = level.as_deref() == Some("forbid")
                || (inherits && workspace_lints_forbid)
                || (!roots.is_empty() && roots.iter().all(|r| crate_root_forbids(r)));
            if !forbidden {
                finding(&manifest_path, "unsafe_code is not forbidden".to_owned());
            }
        }
        let mut files = Vec::new();
        rust_files(&directory, &mut files);
        files.sort();
        for file in files {
            let Ok(text) = fs::read_to_string(&file) else {
                finding(&file, "file is unreadable".to_owned());
                continue;
            };
            let Ok(syntax) = syn::parse_file(&text) else {
                finding(&file, "file does not parse as Rust".to_owned());
                continue;
            };
            let relative = file.strip_prefix(&directory).unwrap_or(&file);
            let in_exec = is_jit
                && (relative == Path::new("src/exec.rs") || relative.starts_with("src/exec"));
            let is_lib = is_jit && relative == Path::new("src/lib.rs");
            let mut scan = Scan {
                items: Vec::new(),
                lowering: Vec::new(),
            };
            scan.visit_file(&syntax);
            if !in_exec {
                for item in scan.items {
                    finding(&file, item);
                }
            }
            for (item, on_mod_exec) in scan.lowering {
                if !(is_lib && on_mod_exec) {
                    finding(&file, item);
                }
            }
            if !in_exec && is_jit {
                for item in &syntax.items {
                    if let syn::Item::Fn(function) = item {
                        if function.sig.abi.is_some() {
                            finding(
                                &file,
                                format!("extern fn {} outside exec", function.sig.ident),
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(findings)
}

/// The report text and whether the check passed.
pub(crate) fn report(root: &Path) -> Result<(String, bool)> {
    let findings = check(root)?;
    let mut summary = String::from("FR-568 unsafe-confinement check (ADR-035 CG-4)\n");
    summary.push_str(&format!(
        "  allowed modules: {}\n",
        ALLOWED
            .iter()
            .map(|(package, module)| format!("({package}, {module})"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if findings.is_empty() {
        summary.push_str("  unsafe-confinement: PASS\n");
    } else {
        summary.push_str("  unsafe-confinement: FAIL\n");
        for finding in &findings {
            summary.push_str(&format!(
                "    {}: {} ({})\n",
                finding.package,
                finding.file.display(),
                finding.item
            ));
        }
    }
    Ok((summary, findings.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    const FORBID: &str = "[package]\nname = \"other\"\n[lints.rust]\nunsafe_code = \"forbid\"\n";
    const DENY: &str = "[package]\nname = \"qsl-jit\"\n[lints.rust]\nunsafe_code = \"deny\"\n";

    /// A clean fixture workspace: `qsl-jit` (deny, `unsafe` only in `exec`)
    /// and `other` (forbid).
    fn clean(root: &Path) {
        write(
            root,
            "Cargo.toml",
            "[package]\nname = \"root\"\n[lints.rust]\nunsafe_code = \"forbid\"\n[workspace]\nmembers = [\"qsl-jit\", \"other\"]\n",
        );
        write(root, "qsl-jit/Cargo.toml", DENY);
        write(
            root,
            "qsl-jit/src/lib.rs",
            "#[allow(unsafe_code)]\nmod exec;\nmod other;\n",
        );
        write(root, "qsl-jit/src/exec.rs", "pub fn f() { unsafe { } }\n");
        write(root, "qsl-jit/src/other.rs", "pub fn g() {}\n");
        write(root, "other/Cargo.toml", FORBID);
        write(root, "other/src/lib.rs", "pub fn h() {}\n");
    }

    fn violates(mutate: impl FnOnce(&Path), expected: &str) {
        let dir = tempfile::tempdir().unwrap();
        clean(dir.path());
        mutate(dir.path());
        let findings = check(dir.path()).unwrap();
        assert!(
            findings.iter().any(|f| f.item.contains(expected)),
            "{findings:?}"
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn the_clean_fixture_passes() {
        let dir = tempfile::tempdir().unwrap();
        clean(dir.path());
        assert_eq!(check(dir.path()).unwrap(), vec![]);
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn an_unsafe_block_in_another_qsl_jit_module_fails() {
        violates(
            |root| write(root, "qsl-jit/src/other.rs", "pub fn g() { unsafe { } }\n"),
            "unsafe block",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn an_unsafe_fn_in_another_package_fails() {
        violates(
            |root| write(root, "other/src/lib.rs", "pub unsafe fn h() {}\n"),
            "unsafe fn h",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn an_unsafe_impl_in_a_file_reached_through_path_fails() {
        violates(
            |root| {
                write(
                    root,
                    "other/src/lib.rs",
                    "#[path = \"../x/y.rs\"]\nmod y;\n",
                );
                write(
                    root,
                    "other/x/y.rs",
                    "struct S;\nunsafe impl Send for S {}\n",
                );
            },
            "unsafe impl",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn an_unsafe_block_in_an_included_file_fails() {
        violates(
            |root| {
                write(root, "other/src/lib.rs", "include!(\"inc.rs\");\n");
                write(root, "other/src/inc.rs", "fn i() { unsafe { } }\n");
            },
            "unsafe block",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn allow_unsafe_code_on_a_second_qsl_jit_module_fails() {
        violates(
            |root| {
                write(
                    root,
                    "qsl-jit/src/lib.rs",
                    "#[allow(unsafe_code)]\nmod exec;\n#[allow(unsafe_code)]\nmod other;\n",
                )
            },
            "allow(unsafe_code)",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn qsl_jit_declaring_forbid_fails_and_another_package_declaring_deny_fails() {
        violates(
            |root| write(root, "qsl-jit/Cargo.toml", &DENY.replace("deny", "forbid")),
            "not `deny`",
        );
        violates(
            |root| write(root, "other/Cargo.toml", &FORBID.replace("forbid", "deny")),
            "not forbidden",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn an_extern_c_function_outside_exec_fails() {
        violates(
            |root| write(root, "qsl-jit/src/other.rs", "pub extern \"C\" fn g() {}\n"),
            "extern fn g",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn a_package_that_forbids_only_at_its_crate_root_passes() {
        let dir = tempfile::tempdir().unwrap();
        clean(dir.path());
        write(
            dir.path(),
            "other/Cargo.toml",
            "[package]\nname = \"other\"\n",
        );
        write(
            dir.path(),
            "other/src/lib.rs",
            "#![forbid(unsafe_code)]\npub fn h() {}\n",
        );
        assert_eq!(check(dir.path()).unwrap(), vec![]);
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn an_unparsable_file_fails_closed() {
        violates(
            |root| write(root, "other/src/lib.rs", "fn broken( {\n"),
            "does not parse",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn an_inner_allow_in_a_module_file_fails() {
        violates(
            |root| {
                write(
                    root,
                    "qsl-jit/src/other.rs",
                    "#![allow(unsafe_code)]\npub fn g() {}\n",
                )
            },
            "allow(unsafe_code)",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn an_allow_on_an_impl_or_struct_fails() {
        violates(
            |root| {
                write(
                    root,
                    "other/src/lib.rs",
                    "#[allow(unsafe_code)]\nstruct S;\n",
                )
            },
            "allow(unsafe_code)",
        );
        violates(
            |root| {
                write(
                    root,
                    "other/src/lib.rs",
                    "struct S;\n#[expect(unsafe_code)]\nimpl S {}\n",
                )
            },
            "expect(unsafe_code)",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn workspace_lints_without_a_workspace_table_do_not_count_as_forbid() {
        violates(
            |root| {
                write(
                    root,
                    "other/Cargo.toml",
                    "[package]\nname = \"other\"\n[lints]\nworkspace = true\n",
                )
            },
            "forbid",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn a_missing_member_manifest_fails() {
        violates(
            |root| {
                write(
                    root,
                    "Cargo.toml",
                    "[package]\nname = \"root\"\n[lints.rust]\nunsafe_code = \"forbid\"\n[workspace]\nmembers = [\"qsl-jit\", \"other\", \"ghost\"]\n",
                )
            },
            "missing or unreadable",
        );
    }

    #[trace("TC-1400", "FR-568-AC-1")]
    #[test]
    fn a_glob_member_fails_and_a_multiline_commented_list_parses() {
        violates(
            |root| {
                write(
                    root,
                    "Cargo.toml",
                    "[package]\nname = \"root\"\n[lints.rust]\nunsafe_code = \"forbid\"\n[workspace]\nmembers = [\"*\"]\n",
                )
            },
            "glob",
        );
        let dir = tempfile::tempdir().unwrap();
        clean(dir.path());
        write(
            dir.path(),
            "Cargo.toml",
            "[package]\nname = \"root\"\n[lints.rust]\nunsafe_code = \"forbid\"\n[workspace]\nmembers = [\n  \"qsl-jit\", # the jit\n  \"other\",\n]\n",
        );
        write(dir.path(), "other/src/lib.rs", "pub unsafe fn h() {}\n");
        let findings = check(dir.path()).unwrap();
        assert!(findings.iter().any(|f| f.item.contains("unsafe fn h")));
    }
}
