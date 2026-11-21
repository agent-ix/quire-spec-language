// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-6's symbolic pass over a task product (FSpec FR-418, producer side).
//!
//! The concrete search in `task_product.rs` walks event instants and can only
//! end at `RegionClosureRequired`: it never covers dense release instants.
//! This pass closes the product the way FR-418 requires. It runs the task
//! automaton `fsl-eval` lowers (the lowering the checker shares, as it shares
//! `ModelSystem`) through the engine's own [`Dbm`] and its own enumeration of
//! the finite refined regions, and returns the closed quotient: vertices
//! `(task product key, refined region)`, discrete edges, positive-delay edges
//! and the bounded subtractions the reachable edges apply.
//!
//! Nothing here calls the checker's exploration (`fsl_replay`'s
//! `explore_task_product`) or its zone code: the DBM is the engine's
//! (ADR-026 CF-6), the region walk is written here from FR-418's definition
//! of a refined region, and the quotient's shape (what is a vertex, when time
//! may pass, what an edge's identity is) is decided here. The checker then
//! recomputes all of it; a mistake the two share would have to be in the
//! lowering itself.

use std::cell::Cell;
use std::collections::{BTreeMap, VecDeque};

use filament_exact::{Cancel, CancelCause, Rational};
use fsl_eval::simulation::model::{
    AllowancePhase, CertificationRefusal, ClockAtom, EdgeKind, JobPhase, LocationInfo,
    TaskAutomaton, TaskAutomatonRefusal, TaskEdge, TaskLocation, TaskTemplate,
};
use fsl_eval::simulation::StateKey;
use fsl_replay::{
    TaskArrivalPhase, TaskJobPhase, TaskLiveJob, TaskProductKey, TaskSchedulerControl,
    TaskTransitionTemplate,
};
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, Zero};

use super::dbm::{Bound, Dbm};

/// A clock bounded below and above and the position of the lower bound's
/// guard, as a cycle's advance rule reads it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicEffect {
    /// `(clock, bound, strict)`: the guard requires `clock > bound` when
    /// strict, else `clock >= bound`.
    pub lowers: Vec<(usize, BigInt, bool)>,
    /// `(clock, value)`: reset to a constant.
    pub resets: Vec<(usize, BigInt)>,
    /// Clocks assigned from another clock.
    pub assigned: Vec<usize>,
}

/// The identity of a symbolic edge.
#[derive(Clone, Debug)]
pub enum SymbolicIdentity {
    /// A source transition releasing no task or several at once.
    Source(filament_semantic_value::model_transition::ModelTransition),
    /// A task-scheduler action, or the release of exactly one task.
    Template(TaskTransitionTemplate),
}

/// One vertex of the closed quotient.
#[derive(Clone, Debug)]
pub struct SymbolicVertex {
    /// The task product key octets.
    pub key: StateKey,
    /// The refined region.
    pub zone: Dbm,
    /// Whether the control has taken a deadline miss.
    pub missed: bool,
}

/// A discrete edge of the quotient.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SymbolicEdge {
    /// The source vertex.
    pub from: usize,
    /// The index into [`SymbolicGraph::identities`].
    pub identity: usize,
    /// The target vertex.
    pub to: usize,
    /// The index into [`SymbolicGraph::effects`].
    pub effect: usize,
}

/// One initial symbolic state and the regions it meets.
#[derive(Clone, Debug)]
pub struct SymbolicInitial {
    /// The task product key.
    pub key: StateKey,
    /// The zero valuation held to its invariants.
    pub zone: Dbm,
    /// The vertices of the regions the zone meets.
    pub vertices: Vec<usize>,
}

/// One bounded subtraction a reachable edge applies.
#[derive(Clone, Debug)]
pub struct SymbolicSubtraction {
    /// The edge's template.
    pub template: TaskTransitionTemplate,
    /// The service clock's name.
    pub clock: String,
    /// The scaled constant.
    pub constant: BigInt,
}

/// The engine's closed refined-region quotient of a task automaton.
#[derive(Clone, Debug)]
pub struct SymbolicGraph {
    /// The scale every constant was multiplied by.
    pub scale: BigInt,
    /// The vertices, in discovery order.
    pub vertices: Vec<SymbolicVertex>,
    /// The distinct edge identities.
    pub identities: Vec<SymbolicIdentity>,
    /// The distinct clock effects.
    pub effects: Vec<SymbolicEffect>,
    /// The zero-delay edges, sorted without repeats.
    pub discrete: Vec<SymbolicEdge>,
    /// The positive-delay edges, sorted without repeats.
    pub time: Vec<(usize, usize)>,
    /// The initial symbolic states that admit a valuation.
    pub initials: Vec<SymbolicInitial>,
    /// The bounded subtractions of every reachable edge, once each.
    pub subtractions: Vec<SymbolicSubtraction>,
    /// The scaled region ceiling of each clock.
    pub ceilings: Vec<BigInt>,
    /// The work the exploration spent.
    pub work: u64,
}

/// Why the symbolic pass produced no quotient.
#[derive(Debug)]
pub enum SymbolicStop {
    /// The work limit.
    WorkLimit(u64),
    /// A cancel.
    Cancelled(CancelCause),
    /// The lowering cannot certify the set, or an allocation past a task's
    /// slot bound or a subtraction outside its guarded range is reachable.
    Uncertified,
    /// A deadline miss is reachable, so no certificate states a proof: the
    /// pass stops at the first one and leaves the verdict to the concrete
    /// search. (A miss FR-418 would allow as a time-locked `locked` node is
    /// stopped the same way: a completeness loss only.)
    Missed,
    /// The lowering refused for another reason.
    Refused(TaskAutomatonRefusal),
    /// The engine broke an invariant of its own.
    Fault(&'static str),
}

impl From<TaskAutomatonRefusal> for SymbolicStop {
    fn from(refusal: TaskAutomatonRefusal) -> Self {
        match refusal {
            TaskAutomatonRefusal::Uncertified(_) => Self::Uncertified,
            other => Self::Refused(other),
        }
    }
}

impl From<super::dbm::DbmError> for SymbolicStop {
    fn from(_: super::dbm::DbmError) -> Self {
        Self::Fault("dbm")
    }
}

/// A constant that is not an integer at the current scale.
struct Rescale(BigInt);

enum Failure {
    Stop(SymbolicStop),
    Rescale(Rescale),
}

impl<T: Into<SymbolicStop>> From<T> for Failure {
    fn from(stop: T) -> Self {
        Self::Stop(stop.into())
    }
}

type Done<T> = Result<T, Failure>;

/// The meter: one unit per elementary step, polling `cancel`.
struct Work<'c> {
    used: Cell<u64>,
    limit: u64,
    cancel: &'c Cancel,
}

impl Work<'_> {
    fn charge(&self) -> Result<(), SymbolicStop> {
        if self.cancel.poll() {
            return Err(SymbolicStop::Cancelled(
                self.cancel.cause().unwrap_or(CancelCause::Requested),
            ));
        }
        let used = self.used.get().saturating_add(1);
        self.used.set(used);
        if used > self.limit {
            return Err(SymbolicStop::WorkLimit(self.limit));
        }
        Ok(())
    }
}

/// Explores `automaton` into its closed refined-region quotient under the
/// work limit `max_work`, choosing the least scale that makes every constant
/// an integer.
///
/// # Errors
///
/// [`SymbolicStop`] when the set is not certifiable, a limit or a cancel
/// stops the pass, or the lowering refuses.
pub fn explore(
    automaton: &TaskAutomaton<'_, '_, '_>,
    max_work: u64,
    cancel: &Cancel,
) -> Result<SymbolicGraph, SymbolicStop> {
    match automaton.certification() {
        Ok(()) => {}
        Err(
            CertificationRefusal::DualWcetModeSwitch
            | CertificationRefusal::NotFixedWork
            | CertificationRefusal::EqualPriorities
            | CertificationRefusal::Jitter,
        ) => return Err(SymbolicStop::Uncertified),
    }
    let work = Work {
        used: Cell::new(0),
        limit: max_work,
        cancel,
    };
    let mut scale = BigInt::one();
    loop {
        match Explorer::run(automaton, &scale, &work) {
            Ok(graph) => return Ok(graph),
            Err(Failure::Stop(stop)) => return Err(stop),
            Err(Failure::Rescale(Rescale(denominator))) => {
                scale = scale.lcm(&denominator);
            }
        }
    }
}

/// `value` in units of `1 / scale`, or the denominator that does not divide
/// the scale.
fn scaled(value: &Rational, scale: &BigInt) -> Result<BigInt, Rescale> {
    let denominator = value.denominator().as_big();
    if (scale % denominator).is_zero() {
        Ok(value.numerator().as_big() * (scale / denominator))
    } else {
        Err(Rescale(denominator.clone()))
    }
}

/// Where one clock lies against its ceiling.
#[derive(Clone)]
enum Place {
    Whole,
    Open(BigInt),
    Saturated,
}

/// A declared diagonal predicate `left - right ≺ value`.
struct Diagonal {
    left: usize,
    right: usize,
    value: BigInt,
    strict: bool,
}

/// A constraint `x_i - x_j ≺ value`.
type Cut = (usize, usize, BigInt, bool);

fn bound_of(cut: &Cut) -> Bound {
    if cut.3 {
        Bound::lt(cut.2.clone())
    } else {
        Bound::le(cut.2.clone())
    }
}

struct Explorer<'t, 's, 'a, 'c> {
    automaton: &'t TaskAutomaton<'t, 's, 'a>,
    scale: &'c BigInt,
    work: &'c Work<'c>,
    dim: usize,
    ceilings: Vec<BigInt>,
    diagonals: Vec<Diagonal>,
    data: BTreeMap<StateKey, std::rc::Rc<LocationData>>,
    locations: Vec<TaskLocation>,
    index: BTreeMap<(StateKey, Fingerprint), usize>,
    graph: SymbolicGraph,
}

struct LocationData {
    info: LocationInfo,
    edges: Option<Vec<TaskEdge>>,
    overflows: Vec<fsl_eval::simulation::model::Overflow>,
}

/// A zone's bounds, row by row, `None` for infinity.
type Fingerprint = Vec<Option<(BigInt, bool)>>;

fn fingerprint(zone: &Dbm) -> Fingerprint {
    let dim = zone.dim();
    let mut cells = Vec::with_capacity(dim * dim);
    for i in 0..dim {
        for j in 0..dim {
            cells.push(match zone.bound(i, j) {
                Ok(Bound::Finite { value, strict }) => Some((value.clone(), *strict)),
                _ => None,
            });
        }
    }
    cells
}

impl<'t, 's, 'a, 'c> Explorer<'t, 's, 'a, 'c> {
    fn run(
        automaton: &'t TaskAutomaton<'t, 's, 'a>,
        scale: &'c BigInt,
        work: &'c Work<'c>,
    ) -> Done<SymbolicGraph> {
        let mut ceilings = Vec::new();
        for clock in automaton.clocks() {
            ceilings.push(scaled(&clock.ceiling, scale).map_err(Failure::Rescale)?);
        }
        let mut diagonals = Vec::new();
        for diagonal in automaton.diagonals() {
            diagonals.push(Diagonal {
                left: diagonal.left,
                right: diagonal.right,
                value: scaled(&diagonal.constant, scale).map_err(Failure::Rescale)?,
                // The automaton states `left - right > c` (or `>=`); the
                // region keeps the truth of its complement `left - right <= c`.
                strict: !diagonal.strict,
            });
        }
        let mut explorer = Explorer {
            automaton,
            scale,
            work,
            dim: automaton.clocks().len() + 1,
            ceilings: ceilings.clone(),
            diagonals,
            data: BTreeMap::new(),
            locations: Vec::new(),
            index: BTreeMap::new(),
            graph: SymbolicGraph {
                scale: scale.clone(),
                vertices: Vec::new(),
                identities: Vec::new(),
                effects: Vec::new(),
                discrete: Vec::new(),
                time: Vec::new(),
                initials: Vec::new(),
                subtractions: Vec::new(),
                ceilings,
                work: 0,
            },
        };
        let mut queue = explorer.initial()?;
        while let Some(at) = queue.pop_front() {
            explorer.expand(at, &mut queue)?;
        }
        let mut graph = explorer.graph;
        graph.discrete.sort_unstable();
        graph.discrete.dedup();
        graph.time.sort_unstable();
        graph.time.dedup();
        graph.work = work.used.get();
        Ok(graph)
    }

    fn spend(&self) -> Done<()> {
        self.work.charge().map_err(Failure::Stop)
    }

    fn sc(&self, value: &Rational) -> Done<BigInt> {
        scaled(value, self.scale).map_err(Failure::Rescale)
    }

    fn constrain(&self, zone: &mut Dbm, atoms: &[ClockAtom]) -> Done<()> {
        for atom in atoms {
            let value = self.sc(&atom.bound)?;
            zone.constrain(
                atom.left,
                atom.right,
                if atom.strict {
                    Bound::lt(value)
                } else {
                    Bound::le(value)
                },
            )?;
        }
        Ok(())
    }

    /// The refined regions the zone meets: each clock's class up to its
    /// ceiling, the order of the fractional parts of the open clocks, then
    /// the truth of every declared diagonal.
    fn regions(&self, zone: &Dbm) -> Done<Vec<Dbm>> {
        let mut out = Vec::new();
        if zone.is_empty() {
            return Ok(out);
        }
        self.walk_clock(
            1,
            Dbm::universe(self.dim),
            zone.clone(),
            &mut Vec::new(),
            &mut out,
        )?;
        Ok(out)
    }

    /// Narrows a region and its part of the zone by `cuts`; `None` when the
    /// part is empty. One unit of work per call.
    fn narrow(&self, region: &Dbm, meet: &Dbm, cuts: &[Cut]) -> Done<Option<(Dbm, Dbm)>> {
        self.spend()?;
        let (mut region, mut meet) = (region.clone(), meet.clone());
        for cut in cuts {
            meet.constrain(cut.0, cut.1, bound_of(cut))?;
            if meet.is_empty() {
                return Ok(None);
            }
            region.constrain(cut.0, cut.1, bound_of(cut))?;
        }
        Ok(Some((region, meet)))
    }

    fn walk_clock(
        &self,
        clock: usize,
        region: Dbm,
        meet: Dbm,
        places: &mut Vec<Place>,
        out: &mut Vec<Dbm>,
    ) -> Done<()> {
        if clock == self.dim {
            return self.walk_fractions(places, region, meet, out);
        }
        let ceiling = self.ceilings[clock - 1].clone();
        let lowest = match meet.bound(0, clock)? {
            Bound::Finite { value, .. } if -value > BigInt::zero() => -value,
            _ => BigInt::zero(),
        };
        let highest = match meet.bound(clock, 0)? {
            Bound::Finite { value, .. } => Some(value.clone()),
            Bound::Infinite => None,
        };
        let mut k = if lowest < ceiling {
            lowest
        } else {
            ceiling.clone()
        };
        loop {
            let whole = [(clock, 0, k.clone(), false), (0, clock, -k.clone(), false)];
            if let Some((r, m)) = self.narrow(&region, &meet, &whole)? {
                places.push(Place::Whole);
                self.walk_clock(clock + 1, r, m, places, out)?;
                places.pop();
            }
            if k >= ceiling {
                let above = [(0, clock, -ceiling.clone(), true)];
                if let Some((r, m)) = self.narrow(&region, &meet, &above)? {
                    places.push(Place::Saturated);
                    self.walk_clock(clock + 1, r, m, places, out)?;
                    places.pop();
                }
                return Ok(());
            }
            let upper = &k + BigInt::one();
            let open = [
                (0, clock, -k.clone(), true),
                (clock, 0, upper.clone(), true),
            ];
            if let Some((r, m)) = self.narrow(&region, &meet, &open)? {
                places.push(Place::Open(k.clone()));
                self.walk_clock(clock + 1, r, m, places, out)?;
                places.pop();
            }
            if highest.as_ref().is_some_and(|top| k >= *top) {
                return Ok(());
            }
            k = upper;
        }
    }

    fn walk_fractions(
        &self,
        places: &[Place],
        region: Dbm,
        meet: Dbm,
        out: &mut Vec<Dbm>,
    ) -> Done<()> {
        let open: Vec<(usize, &BigInt)> = places
            .iter()
            .enumerate()
            .filter_map(|(k, place)| match place {
                Place::Open(base) => Some((k + 1, base)),
                _ => None,
            })
            .collect();
        let mut pairs = Vec::new();
        for (position, &(a, base_a)) in open.iter().enumerate() {
            for &(b, base_b) in &open[position + 1..] {
                pairs.push((a, b, base_a - base_b));
            }
        }
        self.walk_pairs(&pairs, region, meet, out)
    }

    fn walk_pairs(
        &self,
        pairs: &[(usize, usize, BigInt)],
        region: Dbm,
        meet: Dbm,
        out: &mut Vec<Dbm>,
    ) -> Done<()> {
        let Some(((a, b, gap), rest)) = pairs.split_first() else {
            return self.walk_diagonals(0, region, meet, out);
        };
        let minus = -gap.clone();
        let orders: [Vec<Cut>; 3] = [
            vec![(*a, *b, gap.clone(), true)],
            vec![(*a, *b, gap.clone(), false), (*b, *a, minus.clone(), false)],
            vec![(*b, *a, minus, true)],
        ];
        for cuts in orders {
            if let Some((r, m)) = self.narrow(&region, &meet, &cuts)? {
                self.walk_pairs(rest, r, m, out)?;
            }
        }
        Ok(())
    }

    fn walk_diagonals(&self, index: usize, region: Dbm, meet: Dbm, out: &mut Vec<Dbm>) -> Done<()> {
        let Some(diagonal) = self.diagonals.get(index) else {
            out.push(region);
            return Ok(());
        };
        let truths: [Vec<Cut>; 2] = [
            vec![(
                diagonal.left,
                diagonal.right,
                diagonal.value.clone(),
                diagonal.strict,
            )],
            vec![(
                diagonal.right,
                diagonal.left,
                -diagonal.value.clone(),
                !diagonal.strict,
            )],
        ];
        for cuts in truths {
            if let Some((r, m)) = self.narrow(&region, &meet, &cuts)? {
                self.walk_diagonals(index + 1, r, m, out)?;
            }
        }
        Ok(())
    }

    fn location(&mut self, location: &TaskLocation) -> Done<(StateKey, std::rc::Rc<LocationData>)> {
        let key = product_key(location).ok_or(Failure::Stop(SymbolicStop::Fault("key")))?;
        if let Some(found) = self.data.get(&key) {
            return Ok((key, std::rc::Rc::clone(found)));
        }
        let info = self.automaton.location_info(location)?;
        let (edges, overflows) = if location.control().missed {
            (None, Vec::new())
        } else {
            let successors = self.automaton.successors(location)?;
            (Some(successors.edges), successors.overflows)
        };
        let data = std::rc::Rc::new(LocationData {
            info,
            edges,
            overflows,
        });
        self.data.insert(key.clone(), std::rc::Rc::clone(&data));
        Ok((key, data))
    }

    fn vertex(
        &mut self,
        location: &TaskLocation,
        key: &StateKey,
        zone: Dbm,
    ) -> Done<(usize, bool)> {
        let print = (key.clone(), fingerprint(&zone));
        if let Some(&found) = self.index.get(&print) {
            return Ok((found, false));
        }
        if location.control().missed {
            return Err(Failure::Stop(SymbolicStop::Missed));
        }
        let id = self.graph.vertices.len();
        self.graph.vertices.push(SymbolicVertex {
            key: key.clone(),
            zone,
            missed: location.control().missed,
        });
        self.locations.push(location.clone());
        self.index.insert(print, id);
        Ok((id, true))
    }

    fn identity(&mut self, wanted: SymbolicIdentity) -> usize {
        let same = |known: &SymbolicIdentity| match (known, &wanted) {
            (SymbolicIdentity::Source(left), SymbolicIdentity::Source(right)) => {
                fsl_replay::same_source_transition(left, right)
            }
            (SymbolicIdentity::Template(left), SymbolicIdentity::Template(right)) => {
                left.same_identity(right)
            }
            _ => false,
        };
        if let Some(found) = self.graph.identities.iter().position(same) {
            return found;
        }
        self.graph.identities.push(wanted);
        self.graph.identities.len() - 1
    }

    fn pin_dead(&self, zone: &mut Dbm, info: &LocationInfo) -> Done<()> {
        for clock in 1..self.dim {
            if !info.live[clock] {
                zone.reset(clock, &BigInt::zero())?;
            }
        }
        Ok(())
    }

    fn initial(&mut self) -> Done<VecDeque<usize>> {
        let mut queue = VecDeque::new();
        for (location, valuation) in self.automaton.initial()? {
            self.spend()?;
            let (key, data) = self.location(&location)?;
            let mut zone = Dbm::zero(self.dim);
            for (clock, entry) in valuation.iter().enumerate().skip(1) {
                let value = self.sc(entry)?;
                if !value.is_zero() {
                    zone.reset(clock, &value)?;
                }
            }
            self.constrain(&mut zone, &data.info.invariant)?;
            if zone.is_empty() {
                continue;
            }
            let mut vertices = Vec::new();
            for region in self.regions(&zone)? {
                let (id, new) = self.vertex(&location, &key, region)?;
                vertices.push(id);
                if new {
                    queue.push_back(id);
                }
            }
            self.graph.initials.push(SymbolicInitial {
                key,
                zone,
                vertices,
            });
        }
        Ok(queue)
    }

    fn expand(&mut self, at: usize, queue: &mut VecDeque<usize>) -> Done<()> {
        self.spend()?;
        let location = self.locations[at].clone();
        let zone = self.graph.vertices[at].zone.clone();
        let (key, data) = self.location(&location)?;
        let Some(edges) = &data.edges else {
            return Ok(());
        };
        self.delays(at, &location, &key, &zone, &data.info, queue)?;
        for edge in edges {
            let identity = self.identity(identity_of(edge));
            for alternative in edge.guard.alternatives() {
                self.spend()?;
                let mut post = zone.clone();
                self.constrain(&mut post, alternative)?;
                for subtraction in &edge.subtractions {
                    if post.is_empty() {
                        break;
                    }
                    let amount = self.sc(&subtraction.amount)?;
                    let ceiling = self.sc(&subtraction.ceiling)?;
                    // The subtraction is proved only over `amount <= x <=
                    // ceiling`; a valuation outside it would lose its
                    // completion successor, so the pass stops instead.
                    let mut range = post.clone();
                    range.constrain(0, subtraction.clock, Bound::le(-amount.clone()))?;
                    range.constrain(subtraction.clock, 0, Bound::le(ceiling.clone()))?;
                    if !range.includes(&post)? {
                        return Err(Failure::Stop(SymbolicStop::Uncertified));
                    }
                    post.subtract_bounded(subtraction.clock, &amount, &ceiling)?;
                }
                if post.is_empty() {
                    continue;
                }
                self.rows(edge)?;
                let carries: Vec<(usize, usize)> =
                    edge.carries.iter().map(|c| (c.to, c.from)).collect();
                let mut resets = Vec::new();
                for reset in &edge.resets {
                    resets.push((reset.clock, self.sc(&reset.value)?));
                }
                let effect = self.effect(alternative, &carries, &resets)?;
                post.assign_clocks(&carries, &resets)?;
                let (target_key, target) = self.location(&edge.target)?;
                self.pin_dead(&mut post, &target.info)?;
                self.constrain(&mut post, &target.info.invariant)?;
                for region in self.regions(&post)? {
                    let (to, new) = self.vertex(&edge.target, &target_key, region)?;
                    if new {
                        queue.push_back(to);
                    }
                    self.graph.discrete.push(SymbolicEdge {
                        from: at,
                        identity,
                        to,
                        effect,
                    });
                }
            }
        }
        for overflow in &data.overflows {
            for alternative in overflow.guard.alternatives() {
                let mut post = zone.clone();
                self.constrain(&mut post, alternative)?;
                if !post.is_empty() {
                    return Err(Failure::Stop(SymbolicStop::Uncertified));
                }
            }
        }
        Ok(())
    }

    fn effect(
        &mut self,
        alternative: &[ClockAtom],
        carries: &[(usize, usize)],
        resets: &[(usize, BigInt)],
    ) -> Done<usize> {
        let mut lowers = Vec::new();
        for atom in alternative {
            // `x >= c` is `0 - x <= -c`.
            if atom.left == 0 && atom.right != 0 {
                lowers.push((atom.right, -self.sc(&atom.bound)?, atom.strict));
            }
        }
        let effect = SymbolicEffect {
            lowers,
            resets: resets.to_vec(),
            assigned: carries.iter().map(|(to, _)| *to).collect(),
        };
        if let Some(found) = self.graph.effects.iter().position(|own| own == &effect) {
            return Ok(found);
        }
        self.graph.effects.push(effect);
        Ok(self.graph.effects.len() - 1)
    }

    fn rows(&mut self, edge: &TaskEdge) -> Done<()> {
        for subtraction in &edge.subtractions {
            self.spend()?;
            let row = SymbolicSubtraction {
                template: template_of(&subtraction.template),
                clock: self.automaton.clocks()[subtraction.clock - 1].name.clone(),
                constant: self.sc(&subtraction.amount)?,
            };
            let known = self.graph.subtractions.iter().any(|own| {
                own.clock == row.clock
                    && own.constant == row.constant
                    && own.template.same_identity(&row.template)
            });
            if !known {
                self.graph.subtractions.push(row);
            }
        }
        Ok(())
    }

    /// The positive-delay edges of the region at `at`: no delay from the part
    /// of it where a scheduler action is enabled or the source blocks time.
    fn delays(
        &mut self,
        at: usize,
        location: &TaskLocation,
        key: &StateKey,
        zone: &Dbm,
        info: &LocationInfo,
        queue: &mut VecDeque<usize>,
    ) -> Done<()> {
        if info.time_stopped {
            return Ok(());
        }
        // The region minus every urgent alternative, as a list of convex
        // pieces: against an alternative `a1 & ... & an`, the pieces
        // `rest & !ai` and then `rest & ai`.
        let mut free = vec![zone.clone()];
        for alternative in info.urgent.alternatives() {
            let mut rest_pieces = Vec::new();
            for piece in &free {
                self.spend()?;
                let mut inside = piece.clone();
                for atom in alternative {
                    let value = self.sc(&atom.bound)?;
                    let mut outside = inside.clone();
                    // The complement of `x_l - x_r ≺ c` is
                    // `x_r - x_l ≺' -c`.
                    outside.constrain(
                        atom.right,
                        atom.left,
                        if atom.strict {
                            Bound::le(-value.clone())
                        } else {
                            Bound::lt(-value.clone())
                        },
                    )?;
                    if !outside.is_empty() {
                        rest_pieces.push(outside);
                    }
                    inside.constrain(
                        atom.left,
                        atom.right,
                        if atom.strict {
                            Bound::lt(value)
                        } else {
                            Bound::le(value)
                        },
                    )?;
                    if inside.is_empty() {
                        break;
                    }
                }
            }
            free = rest_pieces;
        }
        if free.is_empty() {
            return Ok(());
        }
        for piece in &free {
            let mut later = piece.clone();
            later.up();
            self.pin_dead(&mut later, info)?;
            self.constrain(&mut later, &info.invariant)?;
            for region in self.regions(&later)? {
                if &region == zone {
                    continue;
                }
                let (to, new) = self.vertex(location, key, region)?;
                if new {
                    queue.push_back(to);
                }
                self.graph.time.push((at, to));
            }
        }
        // A region whose every live clock is saturated lets time pass
        // without leaving it.
        let mut saturated = true;
        for clock in 1..self.dim {
            if !info.live[clock] {
                continue;
            }
            let mut within = zone.clone();
            within.constrain(clock, 0, Bound::le(self.ceilings[clock - 1].clone()))?;
            if !within.is_empty() {
                saturated = false;
                break;
            }
        }
        if saturated {
            self.graph.time.push((at, at));
        }
        Ok(())
    }
}

fn template_of(template: &TaskTemplate) -> TaskTransitionTemplate {
    TaskTransitionTemplate {
        action: template.action,
        task_set: template.task_set.clone(),
        task: template.task.clone(),
        job_rank: template.job_rank,
        slot: template.slot,
        release: template.release.clone(),
        source_transition: template.source_transition.clone(),
        priority: template.priority,
    }
}

fn identity_of(edge: &TaskEdge) -> SymbolicIdentity {
    match &edge.kind {
        EdgeKind::Source {
            transition,
            activations,
        } => match activations.as_slice() {
            [one] => SymbolicIdentity::Template(template_of(one)),
            _ => SymbolicIdentity::Source(transition.clone()),
        },
        EdgeKind::Scheduler(template) => SymbolicIdentity::Template(template_of(template)),
    }
}

/// The complete source-plus-scheduler product key of a location. The queue
/// lists the ready jobs in `(task, rank)` order under every policy: the
/// policy order is carried by the region's diagonal truth, not the key.
fn product_key(location: &TaskLocation) -> Option<StateKey> {
    let control = location.control();
    let arrivals = control
        .arrival_started
        .iter()
        .map(|started| {
            if *started {
                TaskArrivalPhase::Arrived
            } else {
                TaskArrivalPhase::NotArrived
            }
        })
        .collect();
    let mut jobs = Vec::new();
    let mut queue = Vec::new();
    for (position, job) in control.jobs.iter().enumerate() {
        let rank = control.rank(position);
        let phase = match job.phase {
            JobPhase::Pending => TaskJobPhase::Pending,
            JobPhase::Ready => TaskJobPhase::Ready,
            JobPhase::Running => TaskJobPhase::Running,
        };
        if phase == TaskJobPhase::Ready {
            queue.push((job.task_index, rank));
        }
        jobs.push(TaskLiveJob {
            task: job.task_index,
            rank,
            slot: job.slot,
            phase,
            started: job.started,
            allowance: AllowancePhase::Lo,
            release: job.release.clone(),
        });
    }
    queue.sort_unstable();
    TaskProductKey::new(
        location.source_key().clone(),
        TaskSchedulerControl {
            arrivals,
            jobs,
            queue,
            mode: control.mode,
            pending_budget_changes: Vec::new(),
            missed: control.missed,
        },
    )
    .ok()?
    .octets()
    .ok()
}
