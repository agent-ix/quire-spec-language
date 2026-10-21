// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064 through the public library: detection, marks, test exclusion, the
//! allow-list, crate scope and request failures.
use ix_trace_rs::trace;
use qsl_string_edge::{scan, AllowListEntry, EdgeRules, Error, Occurrence};

use crate::{request, Tree, CLEAN, VIOLATION};

fn occurrence(file: &str, line: u32, item: &str, branch_gating: bool) -> Occurrence {
    Occurrence {
        file: file.to_owned(),
        line,
        item: item.to_owned(),
        branch_gating,
    }
}

fn entry(file: &str, item: &str) -> AllowListEntry {
    AllowListEntry {
        file: file.to_owned(),
        item: item.to_owned(),
        reason: "test: display-only comparison".to_owned(),
    }
}

/// FR-064-AC-1: the marked comparison is silent; the unmarked `match` is
/// reported at its file, line and item.
#[trace("TC-162", "FR-064-AC-1")]
#[test]
fn a_marked_comparison_is_silent_and_an_unmarked_match_is_reported() {
    let tree = Tree::new();
    tree.write(
        "src/lib.rs",
        "#[string_edge]\n\
         pub fn intake(kind: &str) -> bool { kind == \"value\" }\n\
         pub fn dispatch(kind: &str) -> u8 {\n\
         match kind {\n\
         \"a\" => 1,\n\
         _ => 0,\n\
         }\n\
         }\n",
    );
    let report = scan(&request(tree.path(), &["src"])).expect("a completed scan");
    assert_eq!(
        report.occurrences,
        [occurrence("src/lib.rs", 5, "dispatch", true)]
    );
    assert!(!report.is_clean());
    assert_eq!(
        report.render(),
        "string-edge: 1 unmarked, unlisted occurrence(s):\n  src/lib.rs:5 (branch-gating)\n"
    );
}

/// FR-064-AC-1: a clean tree reports nothing.
#[trace("TC-162", "FR-064-AC-1")]
#[test]
fn a_tree_without_string_dispatch_is_clean() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    let report = scan(&request(tree.path(), &["src"])).expect("a completed scan");
    assert!(report.is_clean());
    assert_eq!(
        report.render(),
        "string-edge: no unmarked, unlisted string comparison or string match found\n"
    );
}

/// FR-064-AC-2: an allow-list entry at the occurrence's file and item hides
/// it; removing the entry, with the source unchanged, reports it again.
#[trace("TC-162", "FR-064-AC-2")]
#[test]
fn an_allow_listed_occurrence_is_silent_and_removing_the_entry_reports_it() {
    let tree = Tree::new();
    tree.write(
        "src/display.rs",
        "pub fn shown(text: &str) {\n\
         let same = text == \"shown\";\n\
         log(same);\n\
         }\n",
    );
    let mut with_entry = request(tree.path(), &["src"]);
    with_entry.rules = EdgeRules {
        allow_list: vec![entry("src/display.rs", "shown")],
    };
    assert!(scan(&with_entry).expect("scan").is_clean());
    let without = request(tree.path(), &["src"]);
    assert_eq!(
        scan(&without).expect("scan").occurrences,
        [occurrence("src/display.rs", 2, "shown", false)]
    );
}

/// FR-064-AC-2: an entry matches on both file and item.
#[trace("TC-162", "FR-064-AC-2")]
#[test]
fn an_allow_list_entry_for_another_file_or_item_hides_nothing() {
    let tree = Tree::new();
    tree.write(
        "src/display.rs",
        "pub fn shown(text: &str) { let same = text == \"shown\"; log(same); }\n",
    );
    for (file, item) in [("src/other.rs", "shown"), ("src/display.rs", "other")] {
        let mut request = request(tree.path(), &["src"]);
        request.rules = EdgeRules {
            allow_list: vec![entry(file, item)],
        };
        assert_eq!(scan(&request).expect("scan").occurrences.len(), 1);
    }
}

/// FR-064-AC-2 and AC-7: an entry's file is base-relative and is normalised
/// the way report files are, so `./src/../src/display.rs` names the file.
#[trace("TC-162", "FR-064-AC-2", "FR-064-AC-7")]
#[test]
fn an_allow_list_entry_path_is_normalised_against_the_base() {
    let tree = Tree::new();
    tree.write(
        "src/display.rs",
        "pub fn shown(text: &str) { let same = text == \"shown\"; log(same); }\n",
    );
    let absolute = tree.path().join("src/display.rs");
    for file in ["./src/../src/display.rs", absolute.to_str().expect("utf-8")] {
        let mut request = request(tree.path(), &["src"]);
        request.rules = EdgeRules {
            allow_list: vec![entry(file, "shown")],
        };
        assert!(scan(&request).expect("scan").is_clean(), "{file}");
    }
}

/// FR-064-AC-3: a `#[cfg(test)]` module, a file under `tests/` and a file
/// declared `#[cfg(test)] mod x;` are not scanned.
#[trace("TC-162", "FR-064-AC-3")]
#[test]
fn test_code_is_not_scanned() {
    let tree = Tree::new();
    tree.write(
        "src/lib.rs",
        "pub fn clean() {}\n\
         #[cfg(test)]\n\
         mod inline { fn check(k: &str) -> bool { k == \"a\" } }\n\
         #[cfg(test)]\n\
         mod unit;\n",
    );
    tree.write("src/unit.rs", VIOLATION);
    tree.write("src/unit/deeper.rs", VIOLATION);
    tree.write("src/tests/it.rs", VIOLATION);
    tree.write("src/target/generated.rs", VIOLATION);
    tree.write("src/.git/hook.rs", VIOLATION);
    let report = scan(&request(tree.path(), &["src"])).expect("scan");
    assert!(report.is_clean(), "{:?}", report.occurrences);
}

/// FR-064-AC-3: a file is never out of scope by its name alone; a
/// production `tests.rs` or `*_tests.rs` not declared `#[cfg(test)]` is
/// scanned, and `mod.rs`/`lib.rs`/`main.rs` own their directory.
#[trace("TC-162", "FR-064-AC-3")]
#[test]
fn a_file_is_never_skipped_by_its_name_alone() {
    let tree = Tree::new();
    tree.write(
        "src/lib.rs",
        "mod real;\nmod plain_tests;\nmod emit;\n#[cfg(test)]\nmod unit;\n",
    );
    tree.write("src/real.rs", "");
    tree.write("src/unit.rs", VIOLATION);
    tree.write("src/plain_tests.rs", VIOLATION);
    tree.write(
        "src/emit.rs",
        "#[cfg(test)]\nmod checks;\nmod tests;\nmod keep;\n",
    );
    tree.write("src/emit/checks.rs", VIOLATION);
    tree.write("src/emit/checks/deep.rs", VIOLATION);
    tree.write("src/emit/keep.rs", "");
    tree.write("src/emit/tests.rs", VIOLATION);
    tree.write("src/dir/mod.rs", "#[cfg(test)]\nmod hidden;\n");
    tree.write("src/dir/hidden.rs", VIOLATION);
    let report = scan(&request(tree.path(), &["src"])).expect("scan");
    let files: Vec<_> = report
        .occurrences
        .iter()
        .map(|occurrence| occurrence.file.as_str())
        .collect();
    assert_eq!(files, ["src/emit/tests.rs", "src/plain_tests.rs"]);
}

/// FR-064-AC-5: an entry at a branch-gating item is a rule error naming its
/// file and item; an entry at a message-only comparison is accepted.
#[trace("TC-162", "FR-064-AC-5")]
#[test]
fn a_branch_gating_entry_is_refused_and_a_message_only_entry_accepted() {
    let tree = Tree::new();
    tree.write(
        "src/lib.rs",
        "pub fn dispatch(kind: &str) {\n\
         if kind == \"allocation\" {\n\
         do_something();\n\
         }\n\
         }\n\
         pub fn log_kind(kind: &str) {\n\
         let flag = kind == \"other\";\n\
         log(flag);\n\
         }\n",
    );
    let mut gating = request(tree.path(), &["src"]);
    gating.rules = EdgeRules {
        allow_list: vec![entry("src/lib.rs", "dispatch")],
    };
    let error = scan(&gating).expect_err("a refused entry");
    assert!(
        matches!(&error, Error::AllowListGatesABranch { file, item }
            if file == "src/lib.rs" && item == "dispatch"),
        "{error:?}"
    );
    let mut message_only = request(tree.path(), &["src"]);
    message_only.rules = EdgeRules {
        allow_list: vec![entry("src/lib.rs", "log_kind")],
    };
    let report = scan(&message_only).expect("accepted");
    assert_eq!(
        report.occurrences,
        [occurrence("src/lib.rs", 2, "dispatch", true)]
    );
}

/// FR-064-AC-5: none of the five ADR-010 §4.3 production dispatch strings
/// can be allow-listed when it gates a branch.
#[trace("TC-162", "FR-064-AC-5")]
#[test]
fn none_of_the_five_adr010_dispatch_sites_can_be_allow_listed() {
    for literal in [
        "allocation",
        "quire.protocol.finite-global/v1",
        "filament-canonical-json-1",
        "quire.state.authority-adapter",
        "clock:",
    ] {
        let tree = Tree::new();
        tree.write(
            "src/lib.rs",
            &format!("pub fn dispatch(v: &str) {{\n    if v == \"{literal}\" {{\n        go();\n    }}\n}}\n"),
        );
        let mut request = request(tree.path(), &["src"]);
        request.rules = EdgeRules {
            allow_list: vec![entry("src/lib.rs", "dispatch")],
        };
        let error = scan(&request).expect_err(literal);
        assert!(
            matches!(&error, Error::AllowListGatesABranch { item, .. } if item == "dispatch"),
            "{literal}: {error:?}"
        );
    }
}

/// FR-064-AC-5: the `clock:` site is a `strip_prefix`, whose `Some` is the
/// dispatch, so an entry at it is refused too.
#[trace("TC-162", "FR-064-AC-5")]
#[test]
fn a_strip_prefix_site_cannot_be_allow_listed() {
    let tree = Tree::new();
    tree.write(
        "src/lib.rs",
        "pub fn binding(v: &str) -> Option<&str> { v.strip_prefix(\"clock:\") }\n",
    );
    let mut request = request(tree.path(), &["src"]);
    request.rules = EdgeRules {
        allow_list: vec![entry("src/lib.rs", "binding")],
    };
    assert!(matches!(
        scan(&request),
        Err(Error::AllowListGatesABranch { .. })
    ));
}

/// FR-064-AC-7: only the selected roots are scanned, each crate resolves
/// constants in its own scope, and equivalent absolute roots give identical
/// ordered occurrences.
#[trace("TC-162", "FR-064-AC-7")]
#[test]
fn only_selected_roots_are_scanned_with_per_crate_constants() {
    let tree = Tree::new();
    tree.write(
        "reader/src/lib.rs",
        "const KIND: &str = \"k\";\n\
         pub fn read(v: &str) -> bool {\n\
         v == KIND\n\
         }\n",
    );
    tree.write(
        "emitter/src/lib.rs",
        "const KIND: usize = 3;\n\
         pub fn emit(n: usize, v: &str) -> bool {\n\
         n == KIND && v == \"x\"\n\
         }\n",
    );
    tree.write("unselected/src/lib.rs", VIOLATION);
    let relative = scan(&request(tree.path(), &["reader/src", "emitter/src"])).expect("scan");
    assert_eq!(
        relative.occurrences,
        [
            occurrence("emitter/src/lib.rs", 3, "emit", true),
            occurrence("reader/src/lib.rs", 3, "read", false),
        ]
    );
    let reader = tree.path().join("reader/src");
    let emitter = tree.path().join("emitter/src");
    let mut absolute = request(tree.path(), &[]);
    absolute.crate_roots = vec![emitter, reader];
    assert_eq!(scan(&absolute).expect("scan"), relative);
}

/// FR-064-AC-8: removing the display entry restores its occurrence, and an
/// entry at a branch-gating item is a rule error even when that item is
/// marked.
#[trace("TC-162", "FR-064-AC-8")]
#[test]
fn a_marked_branch_gating_item_still_refuses_an_allow_list_entry() {
    let tree = Tree::new();
    tree.write(
        "src/lib.rs",
        "#[qsl_attrs::string_edge]\n\
         pub fn intake(kind: &str) -> bool {\n\
         if kind == \"a\" { return true; }\n\
         false\n\
         }\n",
    );
    assert!(scan(&request(tree.path(), &["src"]))
        .expect("scan")
        .is_clean());
    let mut with_entry = request(tree.path(), &["src"]);
    with_entry.rules = EdgeRules {
        allow_list: vec![entry("src/lib.rs", "intake")],
    };
    assert!(matches!(
        scan(&with_entry),
        Err(Error::AllowListGatesABranch { file, item })
            if file == "src/lib.rs" && item == "intake"
    ));
}

/// FR-064-AC-9: empty, missing, non-directory, duplicate, overlapping and
/// out-of-base roots are typed request errors naming the input.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn invalid_roots_are_typed_errors() {
    let tree = Tree::new();
    tree.write("a/src/lib.rs", CLEAN);
    tree.write("a/src/nested/lib.rs", CLEAN);
    tree.write("a/Cargo.toml", "");
    let outside = Tree::new();
    outside.write("src/lib.rs", CLEAN);
    let outside_root = outside.path().join("src");

    assert!(matches!(
        scan(&request(tree.path(), &[])),
        Err(Error::NoCrateRoots)
    ));
    assert!(matches!(
        scan(&request(tree.path(), &["missing"])),
        Err(Error::RootUnresolved { path, .. }) if path.ends_with("missing")
    ));
    assert!(matches!(
        scan(&request(tree.path(), &["a/Cargo.toml"])),
        Err(Error::RootNotDirectory { path }) if path.ends_with("a/Cargo.toml")
    ));
    assert!(matches!(
        scan(&request(tree.path(), &["a/src", "a/src/../src"])),
        Err(Error::RootDuplicate { path }) if path.ends_with("a/src/../src")
    ));
    assert!(matches!(
        scan(&request(tree.path(), &["a/src", "a/src/nested"])),
        Err(Error::RootOverlap { outer, inner })
            if outer.ends_with("a/src") && inner.ends_with("a/src/nested")
    ));
    assert!(matches!(
        scan(&request(tree.path(), &["a/src/nested", "a/src"])),
        Err(Error::RootOverlap { outer, inner })
            if outer.ends_with("a/src") && inner.ends_with("a/src/nested")
    ));
    let mut escaping = request(tree.path(), &[]);
    escaping.crate_roots = vec![outside_root];
    assert!(matches!(
        scan(&escaping),
        Err(Error::RootOutsideBase { .. })
    ));
    assert!(matches!(
        scan(&request(tree.path(), &["../outside"])),
        Err(Error::RootUnresolved { .. } | Error::RootOutsideBase { .. })
    ));
}

/// FR-064-AC-9: a missing or non-directory base is a typed error.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn an_invalid_base_directory_is_a_typed_error() {
    let tree = Tree::new();
    let file = tree.write("file", "");
    assert!(matches!(
        scan(&request(&tree.path().join("absent"), &["src"])),
        Err(Error::BaseDirectory { path, .. }) if path.ends_with("absent")
    ));
    assert!(matches!(
        scan(&request(&file, &["src"])),
        Err(Error::BaseNotDirectory { path }) if path == file
    ));
}

/// FR-064-AC-9: a root with no production Rust file (empty, only tests, or
/// only a `#[cfg(test)]` module file) cannot be clean.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn a_selection_without_production_files_is_an_error() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.path().join("empty")).expect("mkdir");
    tree.write("only_tests/src/tests/it.rs", VIOLATION);
    tree.write("cfg_only/src/lib.rs", "#[cfg(test)]\nmod unit;\n");
    tree.write("cfg_only/src/unit.rs", VIOLATION);
    tree.write("not_rust/src/readme.txt", "x");
    for root in ["empty", "only_tests/src", "not_rust/src"] {
        assert!(
            matches!(
                scan(&request(tree.path(), &[root])),
                Err(Error::NoProductionSources { path }) if path.ends_with(root)
            ),
            "{root}"
        );
    }
    // `lib.rs` itself is production source, so `cfg_only` holds one file.
    assert!(scan(&request(tree.path(), &["cfg_only/src"]))
        .expect("scan")
        .is_clean());
}

/// FR-064-AC-9: malformed Rust and unreadable source are typed errors naming
/// the file; neither yields a clean result, even beside a clean root.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn malformed_and_unreadable_source_are_typed_errors() {
    let tree = Tree::new();
    tree.write("good/src/lib.rs", CLEAN);
    tree.write("bad/src/lib.rs", "fn broken( {\n");
    let error = scan(&request(tree.path(), &["good/src", "bad/src"])).expect_err("parse error");
    assert!(
        matches!(&error, Error::Parse { path, .. } if path.ends_with("bad/src/lib.rs")),
        "{error:?}"
    );

    let tree = Tree::new();
    let binary = tree.path().join("src/lib.rs");
    tree.write("src/lib.rs", "");
    std::fs::write(&binary, [0xff, 0xfe, 0xfd]).expect("write invalid UTF-8");
    let error = scan(&request(tree.path(), &["src"])).expect_err("read error");
    assert!(
        matches!(&error, Error::Read { path, .. } if *path == binary),
        "{error:?}"
    );
}

/// FR-064-AC-9: an unreadable file (permissions) is a read error. Skipped
/// when the test user can read it anyway (root).
#[cfg(unix)]
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn a_file_without_read_permission_is_a_typed_error() {
    use std::os::unix::fs::PermissionsExt;
    let tree = Tree::new();
    let file = tree.write("src/lib.rs", CLEAN);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    if std::fs::read(&file).is_ok() {
        eprintln!("skipped: the test user can read a mode-000 file");
        return;
    }
    let error = scan(&request(tree.path(), &["src"])).expect_err("read error");
    assert!(
        matches!(&error, Error::Read { path, .. } if *path == file),
        "{error:?}"
    );
}
