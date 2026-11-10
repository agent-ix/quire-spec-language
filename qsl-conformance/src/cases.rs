// SPDX-License-Identifier: AGPL-3.0-or-later
//! The matrix table (FR-564): each cell's inputs, as this crate's own bytes.

use qsl_analyze::analyze::DeadlineSupply;
use qsl_replay::ToolFailure;
use quire_exact::{CancelCause, ScalarLimits};
use quire_semantic_value::catalog::Category;

use crate::matrix::{
    AnalyzeInput, ArgumentValue, BackendPlan, Cancellation, CaseInput, EngineItem, ExecuteArgument,
    ExecuteInput, FrontEndInput, FrontEndLimit, Input, MatrixCase, MatrixCell, MonitorInput,
    OutcomeKind, PluginSettlement, PopulationInput, ProveInput, RequestParameters, SubjectInput,
    Verb, WorkingDirectory,
};

/// Ample accounting for every fixture of the matrix.
const AMPLE: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

/// The parameters every case runs under unless its cell says otherwise.
const PARAMETERS: RequestParameters = RequestParameters {
    source_authority: "qsl-conformance",
    source_identity: "lifecycle",
    package_name: "conformance",
    accounting: AMPLE,
    max_window_values: 1 << 20,
    statistical_seed: 7,
    max_witnesses: 3,
    sampler: ("agent-ix", "quire.simulation.sampler/v1"),
    max_states: None,
};

/// `cell` with `parameters`, for the cell that states its own limit.
const fn under(cell: MatrixCell, parameters: RequestParameters) -> MatrixCell {
    match cell {
        MatrixCell::Case(case) => MatrixCell::Case(MatrixCase { parameters, ..case }),
        other => other,
    }
}

/// FR-283-AC-12's `work_units` 0.
const NO_WORK: RequestParameters = RequestParameters {
    accounting: ScalarLimits {
        work_units: 0,
        ..AMPLE
    },
    ..PARAMETERS
};

/// FR-100-AC-6's `work_units` 0 and FR-281-AC-6 (a)'s `max_states` 1.
const ONE_STATE: RequestParameters = RequestParameters {
    max_states: Some(1),
    ..PARAMETERS
};

const fn input(file_name: &'static str, bytes: &'static [u8]) -> Input {
    Input { file_name, bytes }
}

/// FR-100-AC-1's `seven` and FR-100-AC-5's `id`.
const SPINE_RUN: Input = input(
    "spine-run.native",
    include_bytes!("../../tests/fixtures/spine-run.native"),
);

const fn execute_input(
    function: &'static str,
    arguments: &'static [ExecuteArgument],
    backend: BackendPlan,
) -> CaseInput {
    CaseInput::Execute(ExecuteInput {
        unit: SPINE_RUN,
        function,
        arguments,
        backend,
    })
}

/// `id(12)`: 12 is outside `Digit`.
const ID_12: &[ExecuteArgument] = &[ExecuteArgument {
    parameter: "x",
    value: ArgumentValue::Integer(12),
}];

/// FR-275-AC-1's source: a record and two functions, one of which is `seven`.
const COMPILE: Input = input(
    "spine-compile.native",
    include_bytes!("../../tests/fixtures/spine-compile.native"),
);

/// FR-100-AC-5's `inv`: `1 / x` over an `Int[0, 9]` is `ill_typed`.
const ILL_TYPED: Input = input(
    "ill-typed.native",
    include_bytes!("../cases/ill-typed.native"),
);

/// FR-218-AC-5's `repeat` whose body is only a `check`, over the counter
/// model.
const REPEAT_NO_PROGRESS: Input = input(
    "repeat-no-progress.native",
    include_bytes!("../cases/repeat-no-progress.native"),
);

/// FR-185-AC-7's `Service`: checks at S3 and refuses at S4.
const SERVICE: Input = input(
    "service-unit.native",
    include_bytes!("../cases/service-unit.native"),
);

/// `Service` with the probabilistic claim `Two` (FR-188).
const SERVICE_CLAIM: Input = input(
    "service-claim.native",
    include_bytes!("../cases/service-claim.native"),
);

/// The counter model with the temporal clauses `P`, `Q` and `F`.
const COUNTER_MONITOR: Input = input(
    "counter-monitor.native",
    include_bytes!("../cases/counter-monitor.native"),
);

/// The Counter subject (FR-564 "The analyze fixtures"): `model M =
/// "test/counter-step"` and the clause `P`, which holds at values 0 and 1 and
/// breaks at 2.
const COUNTER_STEP: Input = input(
    "counter-step.native",
    include_bytes!("../cases/counter-step.native"),
);
const COUNTER_STEP_PACKAGE: Input = input(
    "counter-step-package.json",
    include_bytes!("../cases/counter-step-package.json"),
);
const COUNTER_STEP_V0: Input = input(
    "counter-step-v0.json",
    include_bytes!("../cases/counter-step-v0.json"),
);
/// FR-281-AC-11's model: `setTo(n: Integer)` modifies an unbounded field.
const COUNTER_UNBOUNDED: Input = input(
    "counter-unbounded.native",
    include_bytes!("../cases/counter-unbounded.native"),
);
const COUNTER_UNBOUNDED_PACKAGE: Input = input(
    "counter-unbounded-package.json",
    include_bytes!("../cases/counter-unbounded-package.json"),
);
const COUNTER_UNBOUNDED_V0: Input = input(
    "counter-unbounded-v0.json",
    include_bytes!("../cases/counter-unbounded-v0.json"),
);
/// The Link subject (ADR-028 §15.4): `probability >= 0.95 [ eventually
/// holds(m.delivered) ]` under every scheduler.
const LINK: Input = input(
    "link-unit.native",
    include_bytes!("../cases/link-unit.native"),
);
const LINK_PACKAGE: Input = input(
    "link-package.json",
    include_bytes!("../cases/link-package.json"),
);
const LINK_S0: Input = input("link-s0.json", include_bytes!("../cases/link-s0.json"));

const SERVICE_PACKAGE: Input = input(
    "service-package.json",
    include_bytes!("../cases/service-package.json"),
);
const COUNTER_PACKAGE: Input = input(
    "counter-package.json",
    include_bytes!("../cases/counter-package.json"),
);
const SERVICE_S0: Input = input(
    "service-s0.json",
    include_bytes!("../cases/service-s0.json"),
);

/// The counter snapshots `v0` to `v3`: one counter `c1` with that `value`.
const COUNTER_SNAPSHOTS: &[Input] = &[
    input(
        "counter-v0.json",
        include_bytes!("../cases/counter-v0.json"),
    ),
    input(
        "counter-v1.json",
        include_bytes!("../cases/counter-v1.json"),
    ),
    input(
        "counter-v2.json",
        include_bytes!("../cases/counter-v2.json"),
    ),
    input(
        "counter-v3.json",
        include_bytes!("../cases/counter-v3.json"),
    ),
    input(
        "counter-out-of-range.json",
        include_bytes!("../cases/counter-out-of-range.json"),
    ),
];

/// A three-position finite trace over `v0` to `v2`.
const TRACE_FINITE: Input = input(
    "trace-finite.json",
    include_bytes!("../cases/trace-finite.json"),
);
/// A lasso of the prefix `v0`, `v1` and the loop `v2`, `v3`.
const TRACE_LASSO: Input = input(
    "trace-lasso.json",
    include_bytes!("../cases/trace-lasso.json"),
);
/// A finite trace whose second position is a snapshot FR-106 admission
/// refuses: its `value` is 9, outside `Int[0, 3]`.
const TRACE_OUT_OF_RANGE: Input = input(
    "trace-out-of-range.json",
    include_bytes!("../cases/trace-out-of-range.json"),
);

const SERVICE_POPULATION: PopulationInput = PopulationInput {
    package: "test/service",
    node: "ix://test/service/servers",
    keys: &["s"],
};

/// FR-280-AC-6: a universe naming a population the package does not declare.
const UNDECLARED_POPULATION: PopulationInput = PopulationInput {
    package: "test/service",
    node: "ix://test/service/ghost",
    keys: &["s"],
};

const COUNTER_POPULATION: PopulationInput = PopulationInput {
    package: "test/counter-step",
    node: "ix://test/counter-step/counters",
    keys: &["c1"],
};

const COUNTER_STEP_SUBJECT: SubjectInput = SubjectInput {
    unit: COUNTER_STEP,
    packages: &[COUNTER_STEP_PACKAGE],
    snapshots: &[COUNTER_STEP_V0],
    initial: &[("test", "v0")],
    population: COUNTER_POPULATION,
};

const COUNTER_UNBOUNDED_SUBJECT: SubjectInput = SubjectInput {
    unit: COUNTER_UNBOUNDED,
    packages: &[COUNTER_UNBOUNDED_PACKAGE],
    snapshots: &[COUNTER_UNBOUNDED_V0],
    initial: &[("test", "v0")],
    population: PopulationInput {
        package: "test/counter-unbounded",
        node: "ix://test/counter-unbounded/counters",
        keys: &["c1"],
    },
};

const LINK_SUBJECT: SubjectInput = SubjectInput {
    unit: LINK,
    packages: &[LINK_PACKAGE],
    snapshots: &[LINK_S0],
    initial: &[("test", "s0")],
    population: PopulationInput {
        package: "test/link",
        node: "ix://test/link/msgs",
        keys: &["m"],
    },
};

const TWO: EngineItem = EngineItem::Statistical { claim: "Two" };
const EN1_ITEM: EngineItem = EngineItem::Clause;
/// A clause item with the `unsupported` disposition (FR-281-AC-3).
const UNSUPPORTED_CLAUSE: EngineItem = EngineItem::UnsupportedClause;
const EN5_ITEM: EngineItem = EngineItem::Exact { claim: "Likely" };

const fn service_subject(population: PopulationInput) -> SubjectInput {
    SubjectInput {
        unit: SERVICE_CLAIM,
        packages: &[SERVICE_PACKAGE],
        snapshots: &[SERVICE_S0],
        initial: &[("test", "s0")],
        population,
    }
}

const fn monitor_input(trace: Input, clauses: &'static [&'static str]) -> MonitorInput {
    MonitorInput {
        unit: COUNTER_MONITOR,
        packages: &[COUNTER_PACKAGE],
        snapshots: COUNTER_SNAPSHOTS,
        trace,
        clauses,
        population: "ix://test/counter/counters",
        key: "c1",
    }
}

/// The `s3.nodes` counter of [`COMPILE`] less one.
const COMPILE_S3_NODES_BELOW: u64 = 10;
/// The `emit.output_bytes` counter of [`COMPILE`]'s package less one.
const COMPILE_OUTPUT_BYTES_BELOW: u64 = 20_275;

const fn front_end(unit: Input, limit: FrontEndLimit) -> FrontEndInput {
    FrontEndInput {
        unit,
        packages: &[],
        limit,
    }
}

const fn case(
    verb: Verb,
    kind: OutcomeKind,
    input: CaseInput,
    cancellation: Cancellation,
    expected_category: Category,
    expected_exit: u8,
) -> MatrixCell {
    MatrixCell::Case(MatrixCase {
        verb,
        kind,
        input,
        parameters: PARAMETERS,
        cancellation,
        working_directory: WorkingDirectory::NewEmpty,
        expected_category,
        expected_exit,
    })
}

const fn not_applicable(verb: Verb, kind: OutcomeKind, reason: &'static str) -> MatrixCell {
    MatrixCell::NotApplicable { verb, kind, reason }
}

/// FR-564 reason 1.
const NO_CLAIM: &str = "`check` and `package` decide no claim.";
/// FR-564 reason 2.
const NO_BACKEND: &str = "The operation runs no backend or tool outside QSL's evaluator. Its \
    internal faults are FR-285's internal failure and are covered there (FR-285-AC-3).";
/// FR-564 reason 4.
const NO_VERDICT: &str = "`execute` returns a value: a `Boolean` `false` is a completed value, \
    category success, and an undefined evaluation is category undefined, which is not a matrix \
    kind. A clause verdict is the clause-run path's (FR-109), reached through `monitor`.";
/// FR-564 reason 3.
const CHECK_REFUSES: &str = "Every refusal other than an unsupported construct is made by \
    `check` (S3) before `package` runs.";

const DEADLINE: Cancellation = Cancellation::BeforeCall(CancelCause::Deadline);
const REQUESTED: Cancellation = Cancellation::BeforeCall(CancelCause::Requested);

static MATRIX: &[MatrixCell] = &[
    // check
    case(
        Verb::Check,
        OutcomeKind::Success,
        CaseInput::Check(front_end(COMPILE, FrontEndLimit::Default)),
        Cancellation::None,
        Category::Success,
        0,
    ),
    not_applicable(Verb::Check, OutcomeKind::Violation, NO_CLAIM),
    case(
        Verb::Check,
        OutcomeKind::Refusal,
        CaseInput::Check(front_end(ILL_TYPED, FrontEndLimit::Default)),
        Cancellation::None,
        Category::Refusal,
        20,
    ),
    case(
        Verb::Check,
        OutcomeKind::Unsupported,
        CaseInput::Check(FrontEndInput {
            unit: REPEAT_NO_PROGRESS,
            packages: &[COUNTER_PACKAGE],
            limit: FrontEndLimit::Default,
        }),
        Cancellation::None,
        Category::Unsupported,
        21,
    ),
    case(
        Verb::Check,
        OutcomeKind::Timeout,
        CaseInput::Check(front_end(COMPILE, FrontEndLimit::Default)),
        DEADLINE,
        Category::Incomplete,
        22,
    ),
    case(
        Verb::Check,
        OutcomeKind::Cancellation,
        CaseInput::Check(front_end(COMPILE, FrontEndLimit::Default)),
        REQUESTED,
        Category::Incomplete,
        22,
    ),
    case(
        Verb::Check,
        OutcomeKind::Incomplete,
        CaseInput::Check(front_end(
            COMPILE,
            FrontEndLimit::S3Nodes(COMPILE_S3_NODES_BELOW),
        )),
        Cancellation::None,
        Category::Incomplete,
        22,
    ),
    not_applicable(Verb::Check, OutcomeKind::ToolFailure, NO_BACKEND),
    // package
    case(
        Verb::Package,
        OutcomeKind::Success,
        CaseInput::Package(front_end(COMPILE, FrontEndLimit::Default)),
        Cancellation::None,
        Category::Success,
        0,
    ),
    not_applicable(Verb::Package, OutcomeKind::Violation, NO_CLAIM),
    not_applicable(Verb::Package, OutcomeKind::Refusal, CHECK_REFUSES),
    case(
        Verb::Package,
        OutcomeKind::Unsupported,
        CaseInput::Package(FrontEndInput {
            unit: SERVICE,
            packages: &[SERVICE_PACKAGE],
            limit: FrontEndLimit::Default,
        }),
        Cancellation::None,
        Category::Unsupported,
        21,
    ),
    case(
        Verb::Package,
        OutcomeKind::Timeout,
        CaseInput::Package(front_end(COMPILE, FrontEndLimit::Default)),
        DEADLINE,
        Category::Incomplete,
        22,
    ),
    case(
        Verb::Package,
        OutcomeKind::Cancellation,
        CaseInput::Package(front_end(COMPILE, FrontEndLimit::Default)),
        REQUESTED,
        Category::Incomplete,
        22,
    ),
    case(
        Verb::Package,
        OutcomeKind::Incomplete,
        CaseInput::Package(front_end(
            COMPILE,
            FrontEndLimit::OutputBytes(COMPILE_OUTPUT_BYTES_BELOW),
        )),
        Cancellation::None,
        Category::Incomplete,
        22,
    ),
    not_applicable(Verb::Package, OutcomeKind::ToolFailure, NO_BACKEND),
    // execute
    case(
        Verb::Execute,
        OutcomeKind::Success,
        execute_input("seven", &[], BackendPlan::Interpreter),
        Cancellation::None,
        Category::Success,
        0,
    ),
    not_applicable(Verb::Execute, OutcomeKind::Violation, NO_VERDICT),
    case(
        Verb::Execute,
        OutcomeKind::Refusal,
        execute_input("id", ID_12, BackendPlan::Interpreter),
        Cancellation::None,
        Category::Refusal,
        20,
    ),
    case(
        Verb::Execute,
        OutcomeKind::Unsupported,
        execute_input("seven", &[], BackendPlan::JitNotSupplied),
        Cancellation::None,
        Category::Unsupported,
        21,
    ),
    case(
        Verb::Execute,
        OutcomeKind::Timeout,
        execute_input("seven", &[], BackendPlan::Interpreter),
        DEADLINE,
        Category::Incomplete,
        22,
    ),
    case(
        Verb::Execute,
        OutcomeKind::Cancellation,
        execute_input("seven", &[], BackendPlan::Interpreter),
        REQUESTED,
        Category::Incomplete,
        22,
    ),
    under(
        case(
            Verb::Execute,
            OutcomeKind::Incomplete,
            execute_input("seven", &[], BackendPlan::Interpreter),
            Cancellation::None,
            Category::Incomplete,
            22,
        ),
        NO_WORK,
    ),
    case(
        Verb::Execute,
        OutcomeKind::ToolFailure,
        execute_input("seven", &[], BackendPlan::FaultInjecting),
        Cancellation::None,
        Category::InternalFailure,
        30,
    ),
    // analyze
    case(
        Verb::Analyze,
        OutcomeKind::Success,
        CaseInput::Analyze(AnalyzeInput::Engine {
            subject: LINK_SUBJECT,
            item: EN5_ITEM,
            deadline: DeadlineSupply::Unbounded,
        }),
        Cancellation::None,
        Category::Success,
        0,
    ),
    case(
        Verb::Analyze,
        OutcomeKind::Violation,
        CaseInput::Analyze(AnalyzeInput::Engine {
            subject: COUNTER_STEP_SUBJECT,
            item: EN1_ITEM,
            deadline: DeadlineSupply::Unbounded,
        }),
        Cancellation::None,
        Category::Violation,
        10,
    ),
    case(
        Verb::Analyze,
        OutcomeKind::Refusal,
        CaseInput::Analyze(AnalyzeInput::Engine {
            subject: service_subject(UNDECLARED_POPULATION),
            item: TWO,
            deadline: DeadlineSupply::Unbounded,
        }),
        Cancellation::None,
        Category::Refusal,
        20,
    ),
    case(
        Verb::Analyze,
        OutcomeKind::Unsupported,
        CaseInput::Analyze(AnalyzeInput::Engine {
            subject: COUNTER_STEP_SUBJECT,
            item: UNSUPPORTED_CLAUSE,
            deadline: DeadlineSupply::Unbounded,
        }),
        Cancellation::None,
        Category::Unsupported,
        21,
    ),
    case(
        Verb::Analyze,
        OutcomeKind::Timeout,
        CaseInput::Analyze(AnalyzeInput::Engine {
            subject: service_subject(SERVICE_POPULATION),
            item: TWO,
            deadline: DeadlineSupply::Milliseconds(1),
        }),
        DEADLINE,
        Category::Incomplete,
        22,
    ),
    case(
        Verb::Analyze,
        OutcomeKind::Cancellation,
        CaseInput::Analyze(AnalyzeInput::Engine {
            subject: service_subject(SERVICE_POPULATION),
            item: TWO,
            deadline: DeadlineSupply::Unbounded,
        }),
        REQUESTED,
        Category::Incomplete,
        22,
    ),
    under(
        case(
            Verb::Analyze,
            OutcomeKind::Incomplete,
            CaseInput::Analyze(AnalyzeInput::Engine {
                subject: COUNTER_STEP_SUBJECT,
                item: EN1_ITEM,
                deadline: DeadlineSupply::Unbounded,
            }),
            Cancellation::None,
            Category::Incomplete,
            22,
        ),
        ONE_STATE,
    ),
    not_applicable(Verb::Analyze, OutcomeKind::ToolFailure, NO_BACKEND),
    // monitor
    case(
        Verb::Monitor,
        OutcomeKind::Success,
        CaseInput::Monitor(monitor_input(TRACE_LASSO, &["Q"])),
        Cancellation::None,
        Category::Success,
        0,
    ),
    case(
        Verb::Monitor,
        OutcomeKind::Violation,
        CaseInput::Monitor(monitor_input(TRACE_FINITE, &["P"])),
        Cancellation::None,
        Category::Violation,
        10,
    ),
    case(
        Verb::Monitor,
        OutcomeKind::Refusal,
        CaseInput::Monitor(monitor_input(TRACE_OUT_OF_RANGE, &["P"])),
        Cancellation::None,
        Category::Refusal,
        20,
    ),
    case(
        Verb::Monitor,
        OutcomeKind::Unsupported,
        CaseInput::Monitor(monitor_input(TRACE_LASSO, &["F"])),
        Cancellation::None,
        Category::Unsupported,
        21,
    ),
    case(
        Verb::Monitor,
        OutcomeKind::Timeout,
        CaseInput::Monitor(monitor_input(TRACE_FINITE, &["P"])),
        DEADLINE,
        Category::Incomplete,
        22,
    ),
    case(
        Verb::Monitor,
        OutcomeKind::Cancellation,
        CaseInput::Monitor(monitor_input(TRACE_FINITE, &["P"])),
        REQUESTED,
        Category::Incomplete,
        22,
    ),
    under(
        case(
            Verb::Monitor,
            OutcomeKind::Incomplete,
            CaseInput::Monitor(monitor_input(TRACE_FINITE, &["P"])),
            Cancellation::None,
            Category::Incomplete,
            22,
        ),
        NO_WORK,
    ),
    not_applicable(Verb::Monitor, OutcomeKind::ToolFailure, NO_BACKEND),
    // prove (QSL half)
    case(
        Verb::Prove,
        OutcomeKind::Success,
        CaseInput::Prove(ProveInput::Provider {
            subject: LINK_SUBJECT,
            item: EN5_ITEM,
        }),
        Cancellation::None,
        Category::Success,
        0,
    ),
    case(
        Verb::Prove,
        OutcomeKind::Violation,
        CaseInput::Prove(ProveInput::Provider {
            subject: COUNTER_STEP_SUBJECT,
            item: EN1_ITEM,
        }),
        Cancellation::None,
        Category::Violation,
        10,
    ),
    case(
        Verb::Prove,
        OutcomeKind::Refusal,
        CaseInput::Prove(ProveInput::Provider {
            subject: service_subject(UNDECLARED_POPULATION),
            item: TWO,
        }),
        Cancellation::None,
        Category::Refusal,
        20,
    ),
    case(
        Verb::Prove,
        OutcomeKind::Unsupported,
        CaseInput::Prove(ProveInput::Provider {
            subject: COUNTER_UNBOUNDED_SUBJECT,
            item: EN1_ITEM,
        }),
        Cancellation::None,
        Category::Unsupported,
        21,
    ),
    case(
        Verb::Prove,
        OutcomeKind::Timeout,
        CaseInput::Prove(ProveInput::Plugin(PluginSettlement::TimedOut)),
        Cancellation::None,
        Category::Incomplete,
        22,
    ),
    case(
        Verb::Prove,
        OutcomeKind::Cancellation,
        CaseInput::Prove(ProveInput::Plugin(PluginSettlement::Cancelled(
            CancelCause::Requested,
        ))),
        Cancellation::None,
        Category::Incomplete,
        22,
    ),
    under(
        case(
            Verb::Prove,
            OutcomeKind::Incomplete,
            CaseInput::Prove(ProveInput::Provider {
                subject: COUNTER_STEP_SUBJECT,
                item: EN1_ITEM,
            }),
            Cancellation::None,
            Category::Incomplete,
            22,
        ),
        ONE_STATE,
    ),
    case(
        Verb::Prove,
        OutcomeKind::ToolFailure,
        CaseInput::Prove(ProveInput::Plugin(PluginSettlement::Failed(
            ToolFailure::Crash,
        ))),
        Cancellation::None,
        Category::InternalFailure,
        30,
    ),
];

/// The matrix cells in FR-564's verb then kind order.
#[must_use]
pub fn lifecycle_matrix() -> &'static [MatrixCell] {
    MATRIX
}
