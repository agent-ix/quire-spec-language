// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-032: one catalog of authored case inputs; expected results stay in tests.

#[derive(Clone, Copy)]
pub(super) struct Row {
    pub key: &'static str,
    pub version: i64,
    pub parent: Option<&'static str>,
}

const ROOT: Row = Row {
    key: "root",
    version: 1,
    parent: None,
};
const CHILD: Row = Row {
    key: "child",
    version: 2,
    parent: Some("root"),
};
const HEALTHY: &[Row] = &[ROOT, CHILD];

#[derive(Clone, Copy)]
pub(super) struct Clause {
    pub name: &'static str,
    pub id: &'static str,
    pub expression: &'static str,
}

const PARENT: Clause = Clause {
    name: "ParentOrder",
    id: "parent_order",
    expression:
        "present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber",
};
const NO_CYCLE: Clause = Clause {
    name: "NoCycle",
    id: "no_cycle",
    expression: "not reaches(self, self, parent)",
};
const SAME_IDENTITY: Clause = Clause {
    name: "SameIdentity",
    id: "same_identity",
    expression: "self = other",
};
const UNCHANGED: Clause = Clause {
    name: "VersionUnchanged",
    id: "version_unchanged",
    expression: "self.versionNumber = pre(self.versionNumber)",
};

#[derive(Clone, Copy)]
pub(super) enum Input {
    Current {
        rows: &'static [Row],
        self_key: &'static str,
        other: Option<&'static str>,
    },
    Update {
        pre: &'static [Row],
        post: &'static [Row],
    },
}

#[derive(Clone, Copy)]
pub(super) struct CaseSpec {
    pub id: &'static str,
    pub clause: Clause,
    pub input: Input,
    pub complete: bool,
    pub include_model: bool,
    pub expression_steps: Option<u64>,
}

const PARENT_CASE: CaseSpec = CaseSpec {
    id: "healthy-parent",
    clause: PARENT,
    input: Input::Current {
        rows: HEALTHY,
        self_key: "child",
        other: None,
    },
    complete: true,
    include_model: true,
    expression_steps: None,
};

// Defining a case adds it to both the enum and generated list; the test oracle's
// exhaustive match must then be extended independently.
macro_rules! cases {
    ($($name:ident => $spec:expr),+ $(,)?) => {
        /// Explicit authored scenarios; expected results live in the integration test.
        #[derive(Clone, Copy, Debug)]
        pub enum Case { $($name),+ }

        /// Every declared case, in deterministic generation order.
        pub const CASES: &[Case] = &[$(Case::$name),+];

        impl Case {
            pub(super) fn spec(self) -> CaseSpec {
                match self { $(Self::$name => $spec),+ }
            }

            /// Stable case-specific identity and output directory.
            pub fn id(self) -> &'static str { self.spec().id }
        }
    };
}

cases! {
    Healthy => PARENT_CASE,
    Violating => CaseSpec {
        id: "violating-parent",
        input: Input::Current {
            rows: &[Row { version: 3, ..ROOT }, CHILD], self_key: "child", other: None,
        },
        ..PARENT_CASE
    },
    Absent => CaseSpec {
        id: "absent-parent",
        input: Input::Current { rows: &[ROOT], self_key: "root", other: None },
        ..PARENT_CASE
    },
    Cycle => CaseSpec {
        id: "cycle", clause: NO_CYCLE,
        input: Input::Current {
            rows: &[Row { parent: Some("child"), ..ROOT }, CHILD],
            self_key: "child", other: None,
        },
        ..PARENT_CASE
    },
    SelfLoop => CaseSpec {
        id: "self-loop", clause: NO_CYCLE,
        input: Input::Current {
            rows: &[Row { parent: Some("child"), ..CHILD }], self_key: "child", other: None,
        },
        ..PARENT_CASE
    },
    Distinct => CaseSpec {
        id: "distinct-identities", clause: SAME_IDENTITY,
        input: Input::Current {
            rows: &[Row { version: 2, ..ROOT }, Row { parent: None, ..CHILD }],
            self_key: "child", other: Some("root"),
        },
        ..PARENT_CASE
    },
    Dangling => CaseSpec {
        id: "dangling-parent",
        input: Input::Current {
            rows: &[ROOT, Row { parent: Some("missing"), ..CHILD }],
            self_key: "child", other: None,
        },
        ..PARENT_CASE
    },
    Incomplete => CaseSpec {
        id: "incomplete-population", complete: false,
        input: Input::Current {
            rows: &[ROOT, Row { parent: Some("missing"), ..CHILD }],
            self_key: "child", other: None,
        },
        ..PARENT_CASE
    },
    MissingModel => CaseSpec { id: "missing-model", include_model: false, ..PARENT_CASE },
    Exhausted => CaseSpec { id: "exhausted-work", expression_steps: Some(0), ..PARENT_CASE },
    Unchanged => CaseSpec {
        id: "unchanged-version", clause: UNCHANGED,
        input: Input::Update { pre: HEALTHY, post: HEALTHY },
        ..PARENT_CASE
    },
    Changed => CaseSpec {
        id: "changed-version", clause: UNCHANGED,
        input: Input::Update { pre: HEALTHY, post: &[ROOT, Row { version: 3, ..CHILD }] },
        ..PARENT_CASE
    },
    ForbiddenParent => CaseSpec {
        id: "forbidden-parent-change", clause: UNCHANGED,
        input: Input::Update { pre: HEALTHY, post: &[ROOT, Row { parent: None, ..CHILD }] },
        ..PARENT_CASE
    },
}
