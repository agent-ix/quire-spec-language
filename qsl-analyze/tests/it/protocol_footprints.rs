// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-216 (TC-661): protocol steps carry static read, write and enabling
//! footprints over model and protocol locations, from which independence and
//! visibility follow (ADR-027 §5 FT-1 to FT-5).
//!
//! A claim's reads are derived from its checked `holds` atoms: `always
//! holds(k.v <= 3)` read for `k` bound to `c` reads (`c`, `v`), and read for
//! `d`, the claim over `d.v`, reads (`d`, `v`).

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_eval::automaton::translate;
use qsl_eval::model_subject::{ProtocolSubject, RoleBinding};
use qsl_eval::simulation::protocol::{
    visible, AnchorName, ClaimReads, ParallelInstance, ProtocolLocation, ProtocolState,
    ProtocolSystem, ProtocolTransition, Sc, StepKind, ThreadPath,
};
use qsl_eval::simulation::{independent, Footprint, Location};
use qsl_package::CheckedPackage;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{ObjectReference, ValueType};

use crate::fill::{
    attempt, contracts, fill_unit, instance, key, model_subject, package, protocol, reachable,
    sc_system, Documents, Reached, PACKAGE,
};
use crate::protocol_system::touch;

/// A unit checked over cells `keys`, each from `v = 0` and `w = 0`.
struct Cells {
    documents: Documents,
    initial: Vec<DocumentRef>,
    checked: CheckedPackage,
    keys: Vec<&'static str>,
}

impl Cells {
    fn new(body: &str, keys: &[&'static str]) -> Self {
        let mut documents = Documents::new();
        let objects: Vec<(&str, i64)> = keys.iter().map(|key| (*key, 0)).collect();
        let initial = vec![documents.start("s0", &objects)];
        Self {
            documents,
            initial,
            checked: package(body),
            keys: keys.to_vec(),
        }
    }

    /// The subject of protocol `name` with `k` bound to `c`.
    fn subject(&self, name: &str) -> ProtocolSubject<'_> {
        ProtocolSubject {
            model: model_subject(
                &self.checked,
                &self.documents,
                self.initial.clone(),
                &self.keys,
            ),
            protocol: name.to_owned(),
            over: instance("c"),
            roles: vec![RoleBinding {
                role: "w".to_owned(),
                model: PACKAGE.to_owned(),
            }],
        }
    }
}

/// The reference of the cell `name`.
fn cell(system: &ProtocolSystem<'_, Sc>, name: &str) -> ObjectReference {
    let ValueType::Reference(cell) = system.protocol().slots.binders[0].value_type else {
        panic!("`k` is a cell");
    };
    system
        .model()
        .bound_reference(cell, &instance(name))
        .expect("a cell of the universe")
}

/// (`object`, `field`).
fn field(object: &ObjectReference, name: &str) -> Location {
    Location::Field {
        object: object.clone(),
        field: key(&format!("Cell/{name}")),
    }
}

/// The membership of `object` in `cells`.
fn member(object: &ObjectReference) -> Location {
    Location::Membership {
        population: key("cells"),
        object: object.clone(),
    }
}

fn ctl(thread: &ThreadPath) -> Location {
    Location::Protocol(ProtocolLocation::Control(thread.clone()))
}

fn root() -> ThreadPath {
    ThreadPath::root(0)
}

/// The branch `label` of `Main/Both`'s instance 0.
fn branch(label: &str) -> ThreadPath {
    root().child("Main/Both", label, 0)
}

/// `inst` of `Main/Both`'s instance 0.
fn both() -> Location {
    Location::Protocol(ProtocolLocation::Instance(ParallelInstance {
        parent: root(),
        node: "Main/Both".to_owned(),
        ordinal: 0,
    }))
}

/// The last segment of `transition`'s node.
fn last(transition: &ProtocolTransition) -> &str {
    transition
        .node()
        .and_then(|node| node.rsplit('/').next())
        .unwrap_or_default()
}

/// Each step of `reached[at]` of `kind` at the node named `node`, with its
/// footprint from that state and the state it reaches.
fn steps<'r>(
    system: &ProtocolSystem<'_, Sc>,
    reached: &'r [Reached],
    at: usize,
    kind: StepKind,
    node: &str,
) -> Vec<(&'r ProtocolTransition, Footprint, usize)> {
    reached[at]
        .steps
        .iter()
        .filter(|(transition, _)| transition.kind() == kind && last(transition) == node)
        .map(|(transition, next)| {
            let footprint = system
                .footprint(&reached[at].state, transition)
                .expect("a footprint");
            (transition, footprint, *next)
        })
        .collect()
}

/// The one step of `reached[at]` of `kind` at `node`.
fn step<'r>(
    system: &ProtocolSystem<'_, Sc>,
    reached: &'r [Reached],
    at: usize,
    kind: StepKind,
    node: &str,
) -> (&'r ProtocolTransition, Footprint, usize) {
    let mut found = steps(system, reached, at, kind, node);
    assert_eq!(found.len(), 1, "one {kind:?} {node} from {at}");
    found.remove(0)
}

/// The state `fork(Both)` reaches from the initial state.
fn forked(system: &ProtocolSystem<'_, Sc>, reached: &[Reached]) -> usize {
    step(system, reached, 0, StepKind::Fork, "Both").2
}

/// The claim `always holds(k.v <= 3)` over a cell.
const BOUNDED: &str = "temporal Bounded using t0 over (k: M::Cell) clock \"model-steps\" \
                       on origin { always holds(k.v <= 3) }\n";

/// What `BOUNDED`'s atoms read for `k` bound to the cell `over` at
/// `state`, derived from its checked clause.
fn bounded(
    unit: &Cells,
    system: &ProtocolSystem<'_, Sc>,
    over: &str,
    state: &ProtocolState<Sc>,
) -> ClaimReads {
    let clause = &unit.checked.graph().temporal_clauses()[0];
    let atoms = translate(clause.clause(), 1 << 20)
        .expect("the clause translates")
        .atoms()
        .to_vec();
    system
        .claim_reads(clause, &atoms, &instance(over), state)
        .expect("the atoms have footprints")
}

/// FR-216-AC-1: in ADR-027 §7, `attempt(A)` and `attempt(B)` at s1 both
/// write (`c`, `v`) and are dependent. Over cells `c` and `d`, `attempt(A)`
/// on `c` and `attempt(B)` applying `setTwo` to `d`, under receiver-scoped
/// frames, are independent; for a claim over `d.v` only, `attempt(A)` is
/// invisible and `attempt(B)` visible.
#[trace("TC-661", "FR-216-AC-1")]
#[test]
fn attempts_on_one_cell_are_dependent_and_on_two_cells_independent() {
    let fill = Cells::new(&fill_unit("self.v = 0", ""), &["c"]);
    let subject = fill.subject("Fill");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let s1 = forked(&system, &reached);
    let c = cell(&system, "c");
    let (_, a, _) = step(&system, &reached, s1, StepKind::Attempt, "A");
    let (_, b, _) = step(&system, &reached, s1, StepKind::Attempt, "B");
    assert!(a.writes.contains(&field(&c, "v")), "{a:?}");
    assert!(b.writes.contains(&field(&c, "v")), "{b:?}");
    assert!(!independent(&a, &b));

    let two = Cells::new(&(fill_unit("self.v = 0", "") + BOUNDED), &["c", "d"]);
    let subject = two.subject("Fill");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let s1 = forked(&system, &reached);
    let (c, d) = (cell(&system, "c"), cell(&system, "d"));
    let on = |node: &str, object: &ObjectReference| {
        let mut found: Vec<(&ProtocolTransition, Footprint)> =
            steps(&system, &reached, s1, StepKind::Attempt, node)
                .into_iter()
                .filter(|(_, footprint, _)| footprint.writes.contains(&field(object, "v")))
                .map(|(transition, footprint, _)| (transition, footprint))
                .collect();
        assert_eq!(found.len(), 1, "one attempt {node} on {object:?}");
        found.remove(0)
    };
    let (a_on_c, a) = on("A", &c);
    let (b_on_d, b) = on("B", &d);
    assert!(!a.writes.contains(&field(&d, "v")), "{a:?}");
    assert!(!b.writes.contains(&field(&c, "v")), "{b:?}");
    assert!(independent(&a, &b), "{a:?} {b:?}");
    let over_d = bounded(&two, &system, "d", &reached[s1].state);
    assert_eq!(over_d.locations, BTreeSet::from([field(&d, "v")]));
    assert!(!visible(a_on_c, &a, &over_d));
    assert!(visible(b_on_d, &b, &over_d));
}

/// FR-216-AC-2: in ADR-027 §7, `fork(Both)` writes `ctl` of the root,
/// `left` and `right` and `inst` of `Both`; `join(Both)` reads `inst` of
/// `Both`; `fork`, `join` and `finish` are invisible to `always holds(k.v <=
/// 3)`. Two steps of one branch, at two states, are dependent.
#[trace("TC-661", "FR-216-AC-2")]
#[test]
fn control_steps_touch_only_protocol_locations_and_one_branch_is_dependent() {
    let fill = Cells::new(&(fill_unit("self.v = 0", "") + BOUNDED), &["c"]);
    let subject = fill.subject("Fill");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let c = cell(&system, "c");
    let (fork, fork_footprint, s1) = step(&system, &reached, 0, StepKind::Fork, "Both");
    let expected: BTreeSet<Location> = [
        ctl(&root()),
        ctl(&branch("left")),
        ctl(&branch("right")),
        both(),
    ]
    .into_iter()
    .collect();
    assert_eq!(fork_footprint.writes, expected);
    // s3, then s4: `B` first, then `A`; s4 joins.
    let s3 = step(&system, &reached, s1, StepKind::Attempt, "B").2;
    let s4 = step(&system, &reached, s3, StepKind::Attempt, "A").2;
    let (join, join_footprint, s5) = step(&system, &reached, s4, StepKind::Join, "Both");
    assert!(join_footprint.reads.contains(&both()), "{join_footprint:?}");
    let (finish, finish_footprint, _) = step(&system, &reached, s5, StepKind::Finish, "Done");
    let claim = bounded(&fill, &system, "c", &reached[0].state);
    assert_eq!(claim.locations, BTreeSet::from([field(&c, "v")]));
    for (transition, footprint) in [
        (fork, &fork_footprint),
        (join, &join_footprint),
        (finish, &finish_footprint),
    ] {
        assert!(!visible(transition, footprint, &claim), "{transition:?}");
    }

    let run = format!(
        "sequence Main {{ parallel Both {{ branch left sequence L {{ {} {} }} branch right {} }} \
         join all [left, right]; }}",
        touch("TA"),
        touch("TB"),
        touch("TC"),
    );
    let two = Cells::new(
        &(contracts("self.v = 0") + &protocol("P", "", &run, "d")),
        &["c"],
    );
    let subject = two.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let s1 = forked(&system, &reached);
    let (_, first, after) = step(&system, &reached, s1, StepKind::Attempt, "TA");
    let (_, second, _) = step(&system, &reached, after, StepKind::Attempt, "TB");
    assert!(first.writes.contains(&ctl(&branch("left"))));
    assert!(!independent(&first, &second));
    // A step of the other branch is independent of both.
    let (_, other, _) = step(&system, &reached, s1, StepKind::Attempt, "TC");
    assert!(independent(&first, &other) && independent(&second, &other));
}

/// The state of `reached` whose root waits at `join Both` with `a`
/// completed and `b` still at its attempt.
fn one_completed(system: &ProtocolSystem<'_, Sc>, reached: &[Reached]) -> usize {
    let s1 = forked(system, reached);
    step(system, reached, s1, StepKind::Attempt, "TA").2
}

/// FR-216-AC-3: under `join any [a, b] outstanding cancel` the join's write
/// footprint holds the `ctl` of every branch thread, under `outstanding
/// continue` none of theirs. A `receive`'s enabling footprint holds its
/// channel's `queue`; in FR-212's `Chatter` the `send` and `receive` on
/// `ch` are dependent, and `attempt(Done)` and `duplicate(ch, Tx)` are
/// independent. `Chatter` is built over this fixture's `Cell`, with `Done`
/// applying `inc` where FR-212's applies `set` to `flag`; its branches,
/// channel premises and nodes are FR-212's.
#[trace("TC-661", "FR-216-AC-3")]
#[test]
fn a_cancelling_join_writes_its_branches_and_a_queue_orders_its_steps() {
    let branches = [branch("a"), branch("b")];
    for (outstanding, cancels) in [("cancel", true), ("continue", false)] {
        let run = format!(
            "sequence Main {{ parallel Both {{ branch a {} branch b {} }} \
             join any [a, b] outstanding {outstanding}; }}",
            touch("TA"),
            touch("TB"),
        );
        let unit = Cells::new(
            &(contracts("self.v = 0") + &protocol("P", "", &run, "d")),
            &["c"],
        );
        let subject = unit.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let at = one_completed(&system, &reached);
        let (_, join, _) = step(&system, &reached, at, StepKind::Join, "Both");
        for thread in &branches {
            assert_eq!(
                join.writes.contains(&ctl(thread)),
                cancels,
                "{outstanding}: {join:?}"
            );
        }
    }

    let chatter = format!(
        "parallel Both {{ branch s sequence Sender {{ send Tx via ch as (tx: Boolean) {{ tx }}; {} }} \
         branch r receive Rx via ch of s::Sender::Tx as (rx: Boolean) {{ true }}; }} join all [s, r];",
        attempt("Done", "inc", "IncPre, IncPost", "done", "true"),
    );
    let members = "channel ch from w to w carries Boolean \
                   ordering unordered delivery unknown capacity 2 overflow block;";
    let unit = Cells::new(
        &(contracts("self.v = 0") + &protocol("Chatter", members, &chatter, "d")),
        &["c"],
    );
    let subject = unit.subject("Chatter");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let s1 = forked(&system, &reached);
    let queue = Location::Protocol(ProtocolLocation::Queue {
        instance: 0,
        channel: "Chatter::ch".to_owned(),
    });
    let (_, send, sent) = step(&system, &reached, s1, StepKind::Send, "Tx");
    let (_, receive, _) = step(&system, &reached, sent, StepKind::Receive, "Rx");
    assert!(receive.enabling.contains(&queue), "{receive:?}");
    assert!(!independent(&send, &receive));
    let (_, done, _) = step(&system, &reached, sent, StepKind::Attempt, "Done");
    let duplicates: Vec<Footprint> = reached[sent]
        .steps
        .iter()
        .filter(|(transition, _)| transition.kind() == StepKind::Duplicate)
        .map(|(transition, _)| {
            system
                .footprint(&reached[sent].state, transition)
                .expect("a footprint")
        })
        .collect();
    assert_eq!(duplicates.len(), 1);
    assert!(
        independent(&done, &duplicates[0]),
        "{done:?} {duplicates:?}"
    );
}

/// FR-216-AC-5: an attempt of `nudge(peer)`, the AC's `bump(by)` (`bump`
/// is this fixture's parameterless operation, and `by` is a keyword of the
/// attempt form), on receiver `c` with reference
/// argument `d`, whose precondition reads `c.v`, whose postcondition reads
/// `pre(d.w)` and whose binder constraint reads binder `a`, has an enabling
/// footprint of exactly (`c`, `v`), (`d`, `w`), the membership of `c` and of
/// `d`, `bind` of `a` and its own `ctl`; a `duplicate(ch, Tx)` step's
/// enabling footprint is exactly `queue(ch)` of its instance. Each protocol
/// location is a `Location::Protocol`.
#[trace("TC-661", "FR-216-AC-5")]
#[test]
fn an_enabling_footprint_holds_exactly_what_enables_the_step() {
    let bump = "pre ByPre using v on M::Cell::nudge { self.v < 3 }\n\
                post ByPost using v on M::Cell::nudge { self.v >= pre(peer.w) }\n";
    let run = "sequence Main { event E by w as (a: Boolean) { a }; \
               attempt X by w on M::Cell::nudge contracts [ByPre, ByPost] as (x: Boolean) { a }; }";
    let unit = Cells::new(
        &(contracts("self.v = 0") + bump + &protocol("P", "", run, "d")),
        &["c", "d"],
    );
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let (c, d) = (cell(&system, "c"), cell(&system, "d"));
    let bound = step(&system, &reached, 0, StepKind::Event, "E").2;
    // One identity, with one step per post-state of `c.v`.
    let on_c_with_d: Vec<(&ProtocolTransition, Footprint)> =
        steps(&system, &reached, bound, StepKind::Attempt, "X")
            .into_iter()
            .map(|(transition, footprint, _)| (transition, footprint))
            .filter(|(_, footprint)| {
                footprint.writes.contains(&field(&c, "v"))
                    && footprint.reads.contains(&field(&d, "w"))
            })
            .collect();
    let (identity, footprint) = on_c_with_d
        .first()
        .cloned()
        .expect("an attempt on c with d");
    assert!(
        on_c_with_d.iter().all(|(other, _)| **other == *identity),
        "{on_c_with_d:?}"
    );
    let binder = Location::Protocol(ProtocolLocation::Binder {
        node: "Main/E".to_owned(),
        thread: root(),
    });
    let expected: BTreeSet<Location> = [
        field(&c, "v"),
        field(&d, "w"),
        member(&c),
        member(&d),
        binder,
        ctl(&root()),
    ]
    .into_iter()
    .collect();
    assert_eq!(footprint.enabling, expected);

    let members = "channel ch from w to w carries Boolean \
                   ordering unordered delivery unknown capacity 2 overflow block;";
    let run = "sequence Main { send Tx via ch as (tx: Boolean) { tx }; }";
    let unit = Cells::new(
        &(contracts("self.v = 0") + &protocol("P", members, run, "d")),
        &["c"],
    );
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let sent = step(&system, &reached, 0, StepKind::Send, "Tx").2;
    let duplicate: &ProtocolTransition = reached[sent]
        .steps
        .iter()
        .map(|(transition, _)| transition)
        .find(|transition| transition.kind() == StepKind::Duplicate)
        .expect("a duplicate step");
    let state: &ProtocolState<Sc> = &reached[sent].state;
    let footprint = system.footprint(state, duplicate).expect("a footprint");
    let queue = Location::Protocol(ProtocolLocation::Queue {
        instance: 0,
        channel: "P::ch".to_owned(),
    });
    assert_eq!(footprint.enabling, BTreeSet::from([queue]));
}

/// ADR-027 FT-1: the step that ends a branch writes that branch's
/// disposition in its parallel instance, so the ends of two sibling branches
/// are independent while each meets the join that reads the whole record;
/// a step of a thread the state does not hold has no footprint.
#[trace("TC-661")]
#[test]
fn a_branch_end_writes_its_own_disposition() {
    let fill = Cells::new(&fill_unit("self.v <= 1", ""), &["c"]);
    let subject = fill.subject("Fill");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let s1 = forked(&system, &reached);
    let (attempt_a, a, s2) = step(&system, &reached, s1, StepKind::Attempt, "A");
    let left = Location::Protocol(ProtocolLocation::Branch {
        instance: ParallelInstance {
            parent: root(),
            node: "Main/Both".to_owned(),
            ordinal: 0,
        },
        label: "left".to_owned(),
    });
    assert!(a.writes.contains(&left), "{a:?}");
    assert!(left.meets(&both()) && !left.meets(&ctl(&branch("left"))));
    // From s2, `A`'s thread has completed: its step is no step of s2.
    assert!(system.footprint(&reached[s2].state, attempt_a).is_err());
}

/// ADR-027 FT-1: an attempt's model reads are those of the clauses its
/// `contracts` select. `setTwo` with `TwoPre` reads (`c`, `v`); with only
/// `TwoPost`, whose post-state read of `v` lies inside its frame, it reads
/// `c`'s membership alone.
#[trace("TC-661")]
#[test]
fn an_attempt_reads_only_the_clauses_it_selects() {
    let reads = |selected: &str| {
        let run = format!(
            "sequence Main {{ {} }}",
            attempt("B", "setTwo", selected, "b", "true")
        );
        let unit = Cells::new(
            &(contracts("self.v = 0") + &protocol("P", "", &run, "d")),
            &["c"],
        );
        let subject = unit.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let c = cell(&system, "c");
        let (_, footprint, _) = step(&system, &reached, 0, StepKind::Attempt, "B");
        let model: BTreeSet<Location> = footprint
            .reads
            .into_iter()
            .filter(|location| !matches!(location, Location::Protocol(_)))
            .collect();
        (model, c)
    };
    let (both_clauses, c) = reads("TwoPre, TwoPost");
    assert_eq!(both_clauses, BTreeSet::from([field(&c, "v"), member(&c)]));
    let (post_only, c) = reads("TwoPost");
    assert_eq!(post_only, BTreeSet::from([member(&c)]));
}

/// The states both orders of the steps `first` then `second` reach from
/// `reached[at]`: every successor of `first`, then every successor of
/// `second` from there.
fn after_both(
    reached: &[Reached],
    at: usize,
    first: &ProtocolTransition,
    second: &ProtocolTransition,
) -> BTreeSet<usize> {
    reached[at]
        .steps
        .iter()
        .filter(|(step, _)| step == first)
        .flat_map(|(_, next)| {
            reached[*next]
                .steps
                .iter()
                .filter(|(step, _)| step == second)
                .map(|(_, last)| *last)
        })
        .collect()
}

/// ADR-027 FT-4 (ADR-021 POR-4): at every reachable state of `system`,
/// every pair of enabled steps reported independent commutes: each stays
/// enabled after the other, and both orders reach the same states. The
/// number of independent pairs checked.
fn assert_independent_steps_commute(system: &ProtocolSystem<'_, Sc>, reached: &[Reached]) -> usize {
    let mut checked = 0;
    for (at, state) in reached.iter().enumerate() {
        let mut identities: Vec<&ProtocolTransition> = Vec::new();
        for (step, _) in &state.steps {
            if !identities.contains(&step) {
                identities.push(step);
            }
        }
        for (index, first) in identities.iter().enumerate() {
            for second in &identities[index + 1..] {
                let left = system.footprint(&state.state, first).expect("a footprint");
                let right = system.footprint(&state.state, second).expect("a footprint");
                if !independent(&left, &right) {
                    continue;
                }
                checked += 1;
                let forward = after_both(reached, at, first, second);
                let backward = after_both(reached, at, second, first);
                assert!(
                    !forward.is_empty() && forward == backward,
                    "from state {at}, {first:?} then {second:?} reach {forward:?}, \
                     the other order {backward:?}"
                );
            }
        }
    }
    checked
}

/// FR-216 and ADR-027 FT-4: over every protocol fixture of TC-661, every
/// pair of steps the footprints report independent commutes. Among them, a
/// `repeat` re-forks `parallel Both` while a branch of the earlier instance
/// still runs under `outstanding continue`: the fork's ordinal depends on
/// whether that branch has ended, so the fork reads the earlier instance.
#[trace("TC-661")]
#[test]
fn every_independent_pair_of_steps_commutes() {
    let parallel = |members: &str| format!("join any [a, b] outstanding {members}");
    let two_touch_left = format!(
        "sequence Main {{ parallel Both {{ branch left sequence L {{ {} {} }} branch right {} }} \
         join all [left, right]; }}",
        touch("TA"),
        touch("TB"),
        touch("TC"),
    );
    let join_any = |outstanding: &str| {
        format!(
            "sequence Main {{ parallel Both {{ branch a {} branch b {} }} {}; }}",
            touch("TA"),
            touch("TB"),
            parallel(outstanding),
        )
    };
    let refork = format!(
        "sequence Main {{ repeat Loop by w visible () max 2 while {{ true }} sequence Body {{ \
         parallel Both {{ branch a {} branch b sequence S {{ {} {} }} }} {}; }} \
         exhausted sequence Out {{ }} }}",
        touch("TA"),
        touch("TB"),
        touch("TC"),
        parallel("continue"),
    );
    let chatter = format!(
        "parallel Both {{ branch s sequence Sender {{ send Tx via ch as (tx: Boolean) {{ tx }}; {} }} \
         branch r receive Rx via ch of s::Sender::Tx as (rx: Boolean) {{ true }}; }} join all [s, r];",
        attempt("Done", "inc", "IncPre, IncPost", "done", "true"),
    );
    let channel = "channel ch from w to w carries Boolean \
                   ordering unordered delivery unknown capacity 2 overflow block;";
    let units: Vec<(String, &str, Vec<&'static str>)> = vec![
        (fill_unit("self.v = 0", ""), "Fill", vec!["c"]),
        (fill_unit("self.v = 0", ""), "Fill", vec!["c", "d"]),
        (fill_unit("self.v <= 1", ""), "Fill", vec!["c", "d"]),
        (
            contracts("self.v = 0") + &protocol("P", "", &two_touch_left, "d"),
            "P",
            vec!["c"],
        ),
        (
            contracts("self.v = 0") + &protocol("P", "", &join_any("cancel"), "d"),
            "P",
            vec!["c"],
        ),
        (
            contracts("self.v = 0") + &protocol("P", "", &join_any("continue"), "d"),
            "P",
            vec!["c"],
        ),
        (
            contracts("self.v = 0") + &protocol("P", "", &refork, "d"),
            "P",
            vec!["c"],
        ),
        (
            contracts("self.v = 0") + &protocol("Chatter", channel, &chatter, "d"),
            "Chatter",
            vec!["c"],
        ),
        (
            contracts("self.v = 0")
                + "pre ByPre using v on M::Cell::nudge { self.v < 3 }\n\
                   post ByPost using v on M::Cell::nudge { self.v >= pre(peer.w) }\n"
                + &protocol(
                    "P",
                    "",
                    &format!(
                        "parallel Both {{ branch a sequence A {{ event E by w as (a: Boolean) {{ a }}; \
                         attempt X by w on M::Cell::nudge contracts [ByPre, ByPost] as (x: Boolean) {{ a }}; }} \
                         branch b {} }} join all [a, b];",
                        touch("TB")
                    ),
                    "d",
                ),
            "P",
            vec!["c", "d"],
        ),
    ];
    let mut total = 0;
    for (index, (body, name, keys)) in units.iter().enumerate() {
        let unit = Cells::new(body, keys);
        let subject = unit.subject(name);
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let checked = assert_independent_steps_commute(&system, &reached);
        // The re-forking `repeat` has independent pairs of its own.
        assert!(index != 6 || checked > 0, "{body}");
        total += checked;
    }
    assert!(total > 0);
}

/// ADR-027 FT-3 and PB-2: an `event` whose record type holds a reference
/// lists its records over the state's objects, so its enabling footprint
/// reads the membership of every population; a step's settling reads the
/// binders of the guards it passes, which do not enable it; and a
/// `duplicate` is visible to an atom that reads its channel's anchor.
#[trace("TC-661")]
#[test]
fn records_guards_and_channel_anchors_are_read() {
    let run = format!(
        "sequence Main {{ event F by w as (f: Boolean) {{ true }}; event E by w as (e: M::Cell) {{ true }}; \
         {} choice Pick by w visible (f) {{ case yes when {{ f }} {} case no when {{ not f }} {} }} }}",
        touch("T"),
        touch("Y"),
        touch("N"),
    );
    let unit = Cells::new(
        &(contracts("self.v = 0") + &protocol("P", "", &run, "d")),
        &["c"],
    );
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let bound = steps(&system, &reached, 0, StepKind::Event, "F")[0].2;
    let any_cell = Location::AnyMembership {
        population: key("cells"),
    };
    let (_, event, listed) = step(&system, &reached, bound, StepKind::Event, "E");
    assert!(event.enabling.contains(&any_cell), "{event:?}");
    let (_, touched, _) = step(&system, &reached, listed, StepKind::Attempt, "T");
    let guard = Location::Protocol(ProtocolLocation::Binder {
        node: "Main/F".to_owned(),
        thread: root(),
    });
    assert!(touched.reads.contains(&guard), "{touched:?}");
    assert!(!touched.enabling.contains(&guard), "{touched:?}");

    let members = "channel ch from w to w carries Boolean \
                   ordering unordered delivery unknown capacity 2 overflow block;";
    let run = "sequence Main { send Tx via ch as (tx: Boolean) { tx }; }";
    let unit = Cells::new(
        &(contracts("self.v = 0") + &protocol("P", members, run, "d")),
        &["c"],
    );
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let sent = step(&system, &reached, 0, StepKind::Send, "Tx").2;
    let (duplicate, _) = reached[sent]
        .steps
        .iter()
        .find(|(transition, _)| transition.kind() == StepKind::Duplicate)
        .expect("a duplicate step");
    let footprint = system
        .footprint(&reached[sent].state, duplicate)
        .expect("a footprint");
    let mut claim = ClaimReads::default();
    assert!(!visible(duplicate, &footprint, &claim));
    claim
        .anchors
        .insert(AnchorName::Channel("P::ch".to_owned()));
    assert!(visible(duplicate, &footprint, &claim));
}
