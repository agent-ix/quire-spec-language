// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-196's canonical semantic action bodies, and the admission of a
//! `FreeDelays` body at a mixed given/free workload race state (FR-202,
//! FR-204).
//!
//! [`ActionBody`] is the closed body form: `["identity",schedule]`,
//! `["post-state",product_key]`, `["terminal-stutter"]`, `["delay"]`,
//! `["enter-idle-delay"]`, `["idle-delay"]`, `["idle-observe"]`,
//! `["workload-resolve",schedule,delay]` and
//! `["free-delays",[[given,free],...]]`, written and read as RFC 8785 JCS
//! octets through `quire-canonical`. Parsing is syntactic only: it checks the
//! octets, the tag, the members, the decimal delays and the schedule order
//! inside each row, and names schedules by their canonical bytes. Whether a
//! body is an enabled choice at a state is the checker's: [`FreeDelaysRace`]
//! admits a `FreeDelays` body against the race the checker rederived, and
//! every refusal is `invalid_runtime_input`/`invalid-value`.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use qsl_eval::simulation::model::{ModelSystem, ScheduledIdentity};
use quire_canonical::{read, to_vec, Document, Encode, Limits, Node, NodeRef, Sink, Writer};
use quire_semantic_value::catalog::CatalogCode;

use crate::probability_certificate::ProbabilityProductKey;

/// Bodies are bounded by the bytes the caller already holds.
const LIMITS: Limits = Limits::new(u64::MAX);

/// The first member of every canonical product key (FR-196).
const PRODUCT_KEY_TAG: &str = "quire.probability-product-key/v1";

/// One row of a `FreeDelays` action (FR-196): a drawn given-delay vector and
/// the scheduler's delay for every free racing identity after that draw, in
/// digital units.
#[derive(Clone, Debug, PartialEq)]
pub struct FreeDelayRow {
    /// Each racing identity whose delay a `discrete` distribution gives, with
    /// its drawn delay.
    pub given: Vec<(ScheduledIdentity, u64)>,
    /// Each racing identity whose delay no distribution gives, with the delay
    /// the scheduler chooses for it.
    pub free: Vec<(ScheduledIdentity, u64)>,
}

/// FR-196's canonical `schedule`, `[operation,receiver,arguments]`, of one
/// scheduled identity: the operation's declaration identity document, the
/// receiver's reference triple and the non-random arguments' typed canonical
/// forms. Equality and order are those of its complete JCS octets.
#[derive(Clone, Debug)]
pub struct Schedule {
    bytes: Vec<u8>,
    document: Document,
}

impl PartialEq for Schedule {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for Schedule {}

impl PartialOrd for Schedule {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Schedule {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl Schedule {
    /// The canonical schedule of `identity`, from FR-181's transition
    /// identity of the identity in `system`.
    ///
    /// # Errors
    ///
    /// [`ActionBodyError::Schedule`] when the identity has no canonical
    /// transition identity in `system`.
    pub fn of(
        system: &ModelSystem<'_>,
        identity: &ScheduledIdentity,
    ) -> Result<Self, ActionBodyError> {
        let transition = system
            .scheduled_identity_key(identity)
            .map_err(|_| ActionBodyError::Schedule)?;
        let transition = read_document(&transition).ok_or(ActionBodyError::Schedule)?;
        let Node::Object(members) = transition.root().node() else {
            return Err(ActionBodyError::Schedule);
        };
        let (mut operation, mut receiver, mut arguments) = (None, None, None);
        for (name, member) in members {
            match name {
                "operation" => operation = Some(member),
                "receiver" => receiver = Some(member),
                "arguments" => arguments = Some(member),
                _ => {}
            }
        }
        let (Some(operation), Some(receiver), Some(arguments)) = (operation, receiver, arguments)
        else {
            return Err(ActionBodyError::Schedule);
        };
        let Node::String(operation) = operation.node() else {
            return Err(ActionBodyError::Schedule);
        };
        let operation = read_document(operation.as_bytes()).ok_or(ActionBodyError::Schedule)?;
        let bytes = to_vec(&Triple([operation.root(), receiver, arguments]), LIMITS)
            .map_err(|_| ActionBodyError::Schedule)?;
        let document = read_document(&bytes).ok_or(ActionBodyError::Schedule)?;
        Ok(Self { bytes, document })
    }

    /// The complete JCS octets.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// A schedule member of a body: an array of exactly an operation, an
    /// object receiver and an argument array.
    fn from_node(node: NodeRef<'_>) -> Result<Self, ActionBodyRefusal> {
        let Node::Array(items) = node.node() else {
            return Err(ActionBodyRefusal::Shape);
        };
        let items: Vec<NodeRef<'_>> = items.collect();
        let [_, receiver, arguments] = items.as_slice() else {
            return Err(ActionBodyRefusal::Shape);
        };
        if !matches!(receiver.node(), Node::Object(_))
            || !matches!(arguments.node(), Node::Array(_))
        {
            return Err(ActionBodyRefusal::Shape);
        }
        let bytes = to_vec(&node, LIMITS).map_err(|_| ActionBodyRefusal::NotCanonical)?;
        let document = read_document(&bytes).ok_or(ActionBodyRefusal::NotCanonical)?;
        Ok(Self { bytes, document })
    }
}

/// One row of a `free-delays` body: `[given,free]`, each a list of
/// `[schedule,delay]` pairs in ascending schedule order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyRow {
    /// The given racing identities with their drawn delays.
    pub given: Vec<(Schedule, u64)>,
    /// The free racing identities with the scheduler's delays.
    pub free: Vec<(Schedule, u64)>,
}

/// FR-196's closed canonical semantic action body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionBody {
    /// `["identity",schedule]`.
    Identity(Schedule),
    /// `["post-state",product_key]`: the complete canonical target key.
    PostState(ProbabilityProductKey),
    /// `["terminal-stutter"]`.
    TerminalStutter,
    /// `["delay"]`: the ordinary running unit delay.
    UnitDelay,
    /// `["enter-idle-delay"]`.
    EnterIdleDelay,
    /// `["idle-delay"]`.
    IdleDelay,
    /// `["idle-observe"]`.
    IdleObserve,
    /// `["workload-resolve",schedule,delay]`.
    WorkloadResolve {
        /// The first identity the workload may resolve to.
        schedule: Schedule,
        /// The digital units of delay before the resolution.
        delay: u64,
    },
    /// `["free-delays",[[given,free],...]]`.
    FreeDelays(Vec<BodyRow>),
}

impl ActionBody {
    /// The body of a `FreeDelays` action over `rows` in `system`: each row's
    /// pairs sorted by schedule, and the rows in ascending numeric order of
    /// their given delays in `given` order (FR-196), whatever order `rows`
    /// holds them in.
    ///
    /// # Errors
    ///
    /// [`ActionBodyError::Schedule`] when an identity has no canonical
    /// schedule.
    pub fn free_delays(
        system: &ModelSystem<'_>,
        rows: &[FreeDelayRow],
    ) -> Result<Self, ActionBodyError> {
        let pairs = |list: &[(ScheduledIdentity, u64)]| {
            let mut pairs = list
                .iter()
                .map(|(identity, delay)| Ok((Schedule::of(system, identity)?, *delay)))
                .collect::<Result<Vec<_>, ActionBodyError>>()?;
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            Ok::<_, ActionBodyError>(pairs)
        };
        let mut body = rows
            .iter()
            .map(|row| {
                Ok(BodyRow {
                    given: pairs(&row.given)?,
                    free: pairs(&row.free)?,
                })
            })
            .collect::<Result<Vec<_>, ActionBodyError>>()?;
        body.sort_by_key(|row| delays(&row.given));
        Ok(Self::FreeDelays(body))
    }

    /// The body's complete JCS octets.
    ///
    /// # Errors
    ///
    /// [`ActionBodyError::Canonical`] when a `PostState` key is not one JSON
    /// document or the body does not encode.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ActionBodyError> {
        let key;
        let view = match self {
            Self::Identity(schedule) => View::Scheduled("identity", schedule),
            Self::PostState(product_key) => {
                key = read_document(product_key.as_bytes()).ok_or(ActionBodyError::Canonical)?;
                View::PostState(key.root())
            }
            Self::TerminalStutter => View::Tag("terminal-stutter"),
            Self::UnitDelay => View::Tag("delay"),
            Self::EnterIdleDelay => View::Tag("enter-idle-delay"),
            Self::IdleDelay => View::Tag("idle-delay"),
            Self::IdleObserve => View::Tag("idle-observe"),
            Self::WorkloadResolve { schedule, delay } => View::Resolve(schedule, *delay),
            Self::FreeDelays(rows) => View::Rows(rows),
        };
        to_vec(&view, LIMITS).map_err(|_| ActionBodyError::Canonical)
    }

    /// Read a body from its complete JCS octets.
    ///
    /// # Errors
    ///
    /// [`ActionBodyRefusal`]: octets that are not one JCS document, an
    /// unknown tag or a missing, extra or mistyped member, a noncanonical
    /// delay, or a row whose `given` or `free` pairs are not in strictly
    /// ascending schedule order.
    pub fn parse(bytes: &[u8]) -> Result<Self, ActionBodyRefusal> {
        let document = read_document(bytes).ok_or(ActionBodyRefusal::NotCanonical)?;
        if to_vec(&document.root(), LIMITS).map_err(|_| ActionBodyRefusal::NotCanonical)? != bytes {
            return Err(ActionBodyRefusal::NotCanonical);
        }
        let items = array(document.root())?;
        let Some((tag, members)) = items.split_first() else {
            return Err(ActionBodyRefusal::Shape);
        };
        let Node::String(tag) = tag.node() else {
            return Err(ActionBodyRefusal::Shape);
        };
        Ok(match (tag, members) {
            ("identity", [schedule]) => Self::Identity(Schedule::from_node(*schedule)?),
            ("post-state", [key]) => Self::PostState(product_key(*key)?),
            ("terminal-stutter", []) => Self::TerminalStutter,
            ("delay", []) => Self::UnitDelay,
            ("enter-idle-delay", []) => Self::EnterIdleDelay,
            ("idle-delay", []) => Self::IdleDelay,
            ("idle-observe", []) => Self::IdleObserve,
            ("workload-resolve", [schedule, delay]) => Self::WorkloadResolve {
                schedule: Schedule::from_node(*schedule)?,
                delay: decimal(*delay)?,
            },
            ("free-delays", [rows]) => Self::FreeDelays(
                array(*rows)?
                    .into_iter()
                    .map(|row| match array(row)?.as_slice() {
                        [given, free] => Ok(BodyRow {
                            given: pairs(*given)?,
                            free: pairs(*free)?,
                        }),
                        _ => Err(ActionBodyRefusal::Shape),
                    })
                    .collect::<Result<_, _>>()?,
            ),
            _ => return Err(ActionBodyRefusal::Shape),
        })
    }
}

/// The given delays of a row, in `given` order.
fn delays(pairs: &[(Schedule, u64)]) -> Vec<u64> {
    pairs.iter().map(|(_, delay)| *delay).collect()
}

fn read_document(bytes: &[u8]) -> Option<Document> {
    read(bytes, u64::try_from(bytes.len()).ok()?).ok()
}

fn array(node: NodeRef<'_>) -> Result<Vec<NodeRef<'_>>, ActionBodyRefusal> {
    match node.node() {
        Node::Array(items) => Ok(items.collect()),
        _ => Err(ActionBodyRefusal::Shape),
    }
}

/// A canonical unsigned decimal string: no sign, no leading zero.
fn decimal(node: NodeRef<'_>) -> Result<u64, ActionBodyRefusal> {
    let Node::String(text) = node.node() else {
        return Err(ActionBodyRefusal::Shape);
    };
    let canonical = !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    if !canonical {
        return Err(ActionBodyRefusal::Delay);
    }
    text.parse().map_err(|_| ActionBodyRefusal::Delay)
}

/// A `[[schedule,delay],...]` list in strictly ascending schedule order.
fn pairs(node: NodeRef<'_>) -> Result<Vec<(Schedule, u64)>, ActionBodyRefusal> {
    let mut pairs: Vec<(Schedule, u64)> = Vec::new();
    for pair in array(node)? {
        let members = array(pair)?;
        let [schedule, delay] = members.as_slice() else {
            return Err(ActionBodyRefusal::Shape);
        };
        let schedule = Schedule::from_node(*schedule)?;
        if pairs.last().is_some_and(|(last, _)| *last >= schedule) {
            return Err(ActionBodyRefusal::ScheduleOrder);
        }
        pairs.push((schedule, decimal(*delay)?));
    }
    Ok(pairs)
}

/// A `post-state` member: a product-key array under its version tag.
fn product_key(node: NodeRef<'_>) -> Result<ProbabilityProductKey, ActionBodyRefusal> {
    let items = array(node)?;
    let tagged = matches!(
        items.first().map(NodeRef::node),
        Some(Node::String(PRODUCT_KEY_TAG))
    );
    if !tagged || items.len() != 6 {
        return Err(ActionBodyRefusal::Shape);
    }
    let bytes = to_vec(&node, LIMITS).map_err(|_| ActionBodyRefusal::NotCanonical)?;
    Ok(ProbabilityProductKey::from_canonical_bytes(bytes))
}

/// `[operation,receiver,arguments]` from three document nodes.
struct Triple<'d>([NodeRef<'d>; 3]);

impl Encode for Triple<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        for node in &self.0 {
            node.encode_into(writer)?;
        }
        writer.end_array()
    }
}

/// What a body writes, with a `PostState` key already read.
enum View<'a> {
    Tag(&'static str),
    Scheduled(&'static str, &'a Schedule),
    PostState(NodeRef<'a>),
    Resolve(&'a Schedule, u64),
    Rows(&'a [BodyRow]),
}

impl Encode for View<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        match self {
            Self::Tag(tag) => writer.string(tag)?,
            Self::Scheduled(tag, schedule) => {
                writer.string(tag)?;
                schedule.document.root().encode_into(writer)?;
            }
            Self::PostState(key) => {
                writer.string("post-state")?;
                key.encode_into(writer)?;
            }
            Self::Resolve(schedule, delay) => {
                writer.string("workload-resolve")?;
                schedule.document.root().encode_into(writer)?;
                writer.string(&delay.to_string())?;
            }
            Self::Rows(rows) => {
                writer.string("free-delays")?;
                writer.begin_array()?;
                for row in *rows {
                    writer.begin_array()?;
                    write_pairs(writer, &row.given)?;
                    write_pairs(writer, &row.free)?;
                    writer.end_array()?;
                }
                writer.end_array()?;
            }
        }
        writer.end_array()
    }
}

fn write_pairs<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    pairs: &[(Schedule, u64)],
) -> Result<(), quire_canonical::Error> {
    writer.begin_array()?;
    for (schedule, delay) in pairs {
        writer.begin_array()?;
        schedule.document.root().encode_into(writer)?;
        writer.string(&delay.to_string())?;
        writer.end_array()?;
    }
    writer.end_array()
}

/// Why a body could not be written.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ActionBodyError {
    /// A scheduled identity has no canonical schedule in the system.
    #[error("a scheduled identity has no canonical schedule")]
    Schedule,
    /// A product key is not one JSON document, or the body does not encode.
    #[error("the action body does not encode")]
    Canonical,
}

/// Why a body is not an enabled choice: every cause refuses
/// `invalid_runtime_input`/`invalid-value` ([`ActionBodyRefusal::code`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ActionBodyRefusal {
    /// The octets are not one JSON document equal to its own JCS encoding.
    #[error("the action body is not canonical JCS")]
    NotCanonical,
    /// An unknown tag, or a missing, extra or mistyped member.
    #[error("the action body has an unknown tag or a malformed member")]
    Shape,
    /// A delay that is not a canonical unsigned decimal string.
    #[error("a delay is not a canonical unsigned decimal string")]
    Delay,
    /// A row's `given` or `free` pairs are not in strictly ascending schedule
    /// order.
    #[error("a row's pairs are not in ascending schedule order")]
    ScheduleOrder,
    /// The body is not a `FreeDelays` action, the only choice of a mixed
    /// given/free race state.
    #[error("the action is not enabled at a mixed race state")]
    Disabled,
    /// A row's `given` does not name exactly the given racing identities.
    #[error("row {row} does not name exactly the given racing identities")]
    GivenIdentities {
        /// The zero-based row.
        row: usize,
    },
    /// A row's `free` does not name exactly the free racing identities.
    #[error("row {row} does not name exactly the free racing identities")]
    FreeIdentities {
        /// The zero-based row.
        row: usize,
    },
    /// A row's given delay is outside its identity's conditioned support.
    #[error("row {row} draws a given delay outside its support")]
    GivenOutsideSupport {
        /// The zero-based row.
        row: usize,
    },
    /// A row's free delay is outside its identity's window at the state.
    #[error("row {row} assigns a free delay outside its window")]
    FreeOutsideWindow {
        /// The zero-based row.
        row: usize,
    },
    /// A row repeats an earlier row's given vector.
    #[error("row {row} repeats a given vector")]
    RowRepeated {
        /// The zero-based row.
        row: usize,
    },
    /// The rows do not cover the conditioned given-delay support.
    #[error("the rows do not cover the given-delay support")]
    RowsMissing,
    /// The rows are not in ascending numeric order of their given delays.
    #[error("row {row} is out of numeric order")]
    RowOrder {
        /// The zero-based row.
        row: usize,
    },
}

impl ActionBodyRefusal {
    /// The catalog code replay refuses with (FR-202).
    pub const fn code(&self) -> CatalogCode {
        CatalogCode::new("invalid_runtime_input", "invalid-value")
    }
}

/// A racing identity whose delay a `discrete` distribution gives.
#[derive(Clone, Debug)]
pub struct GivenRacer {
    /// The identity.
    pub identity: ScheduledIdentity,
    /// Its canonical schedule.
    pub schedule: Schedule,
    /// Its delay support conditioned on its window, in digital units.
    pub support: BTreeSet<u64>,
}

/// A racing identity whose delay no distribution gives.
#[derive(Clone, Debug)]
pub struct FreeRacer {
    /// The identity.
    pub identity: ScheduledIdentity,
    /// Its canonical schedule.
    pub schedule: Schedule,
    /// The delays of its window at the state, in digital units.
    pub window: BTreeSet<u64>,
}

/// A mixed given/free workload race state as the checker rederived it
/// (FR-204): its given racers with their conditioned supports and its free
/// racers with their windows.
#[derive(Clone, Debug)]
pub struct FreeDelaysRace {
    given: Vec<GivenRacer>,
    free: Vec<FreeRacer>,
}

/// A race with no given or no free racer, an empty support or window, or a
/// repeated schedule is not a mixed race.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("not a mixed given/free race")]
pub struct NotMixedRace;

impl FreeDelaysRace {
    /// The race of `given` and `free`, each sorted by schedule.
    ///
    /// # Errors
    ///
    /// [`NotMixedRace`].
    pub fn new(mut given: Vec<GivenRacer>, mut free: Vec<FreeRacer>) -> Result<Self, NotMixedRace> {
        given.sort_by(|left, right| left.schedule.cmp(&right.schedule));
        free.sort_by(|left, right| left.schedule.cmp(&right.schedule));
        let mut schedules: Vec<&Schedule> = given
            .iter()
            .map(|racer| &racer.schedule)
            .chain(free.iter().map(|racer| &racer.schedule))
            .collect();
        let racers = schedules.len();
        schedules.sort();
        schedules.dedup();
        let empty = given.iter().any(|racer| racer.support.is_empty())
            || free.iter().any(|racer| racer.window.is_empty());
        if given.is_empty() || free.is_empty() || empty || schedules.len() != racers {
            return Err(NotMixedRace);
        }
        Ok(Self { given, free })
    }

    /// Admit `body` as the state's `FreeDelays` action (FR-196, FR-202): it
    /// must be a `FreeDelays` body whose rows cover the conditioned
    /// given-delay support exactly once, in ascending numeric order, each
    /// naming exactly the given and the free racing identities, with each
    /// free delay in its identity's window.
    ///
    /// # Errors
    ///
    /// [`ActionBodyRefusal`] naming the first failed condition.
    pub fn admit(&self, body: &ActionBody) -> Result<Vec<FreeDelayRow>, ActionBodyRefusal> {
        let ActionBody::FreeDelays(rows) = body else {
            return Err(ActionBodyRefusal::Disabled);
        };
        let mut admitted = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            let given = self.given_row(index, &row.given)?;
            let free = self.free_row(index, &row.free)?;
            admitted.push(FreeDelayRow { given, free });
        }
        let vectors: Vec<Vec<u64>> = rows.iter().map(|row| delays(&row.given)).collect();
        let mut seen = BTreeSet::new();
        for (index, vector) in vectors.iter().enumerate() {
            if !seen.insert(vector) {
                return Err(ActionBodyRefusal::RowRepeated { row: index });
            }
        }
        let expected = self.given.iter().try_fold(1_usize, |count, racer| {
            count.checked_mul(racer.support.len())
        });
        if expected != Some(vectors.len()) {
            return Err(ActionBodyRefusal::RowsMissing);
        }
        if let Some(index) = vectors
            .windows(2)
            .position(|pair| matches!(pair, [left, right] if left > right))
        {
            return Err(ActionBodyRefusal::RowOrder { row: index + 1 });
        }
        Ok(admitted)
    }

    fn given_row(
        &self,
        row: usize,
        pairs: &[(Schedule, u64)],
    ) -> Result<Vec<(ScheduledIdentity, u64)>, ActionBodyRefusal> {
        if pairs.len() != self.given.len() {
            return Err(ActionBodyRefusal::GivenIdentities { row });
        }
        self.given
            .iter()
            .zip(pairs)
            .map(|(racer, (schedule, delay))| {
                if racer.schedule != *schedule {
                    return Err(ActionBodyRefusal::GivenIdentities { row });
                }
                if !racer.support.contains(delay) {
                    return Err(ActionBodyRefusal::GivenOutsideSupport { row });
                }
                Ok((racer.identity.clone(), *delay))
            })
            .collect()
    }

    fn free_row(
        &self,
        row: usize,
        pairs: &[(Schedule, u64)],
    ) -> Result<Vec<(ScheduledIdentity, u64)>, ActionBodyRefusal> {
        if pairs.len() != self.free.len() {
            return Err(ActionBodyRefusal::FreeIdentities { row });
        }
        self.free
            .iter()
            .zip(pairs)
            .map(|(racer, (schedule, delay))| {
                if racer.schedule != *schedule {
                    return Err(ActionBodyRefusal::FreeIdentities { row });
                }
                if !racer.window.contains(delay) {
                    return Err(ActionBodyRefusal::FreeOutsideWindow { row });
                }
                Ok((racer.identity.clone(), *delay))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    const A: &str = r#"[{"op":"A"},{"o":"1"},[]]"#;
    const B: &str = r#"[{"op":"B"},{"o":"1"},[]]"#;

    fn body(rows: &str) -> String {
        format!(r#"["free-delays",{rows}]"#)
    }

    #[trace("TC-631", "FR-196-AC-14")]
    #[test]
    fn every_body_round_trips_through_its_jcs_octets() {
        let rows = format!(r#"[[[[{A},"9"]],[[{B},"1"]]],[[[{A},"10"]],[[{B},"2"]]]]"#);
        let key = r#"["quire.probability-product-key/v1",null,["m"],null,null,null]"#;
        for text in [
            body(&rows),
            format!(r#"["identity",{A}]"#),
            format!(r#"["post-state",{key}]"#),
            r#"["terminal-stutter"]"#.to_owned(),
            r#"["delay"]"#.to_owned(),
            r#"["enter-idle-delay"]"#.to_owned(),
            r#"["idle-delay"]"#.to_owned(),
            r#"["idle-observe"]"#.to_owned(),
            format!(r#"["workload-resolve",{B},"0"]"#),
        ] {
            let parsed = ActionBody::parse(text.as_bytes()).expect("a canonical body parses");
            assert_eq!(parsed.to_bytes().expect("it writes"), text.as_bytes());
        }
        let ActionBody::FreeDelays(parsed) =
            ActionBody::parse(body(&rows).as_bytes()).expect("parses")
        else {
            panic!("a free-delays body");
        };
        assert_eq!(
            parsed
                .iter()
                .map(|row| delays(&row.given))
                .collect::<Vec<_>>(),
            [vec![9], vec![10]]
        );
        assert_eq!(parsed[1].free[0].1, 2);
    }

    #[trace("TC-631", "FR-196-AC-14")]
    #[test]
    fn malformed_octets_tags_members_and_delays_refuse() {
        let refused = |text: &str| ActionBody::parse(text.as_bytes()).expect_err(text);
        assert_eq!(refused(r#"[ "delay"]"#), ActionBodyRefusal::NotCanonical);
        assert_eq!(refused(r#"["delay""#), ActionBodyRefusal::NotCanonical);
        assert_eq!(refused(r#"["drawn-timed"]"#), ActionBodyRefusal::Shape);
        assert_eq!(refused(r#"["delay","1"]"#), ActionBodyRefusal::Shape);
        assert_eq!(refused(r#"{"tag":"delay"}"#), ActionBodyRefusal::Shape);
        assert_eq!(
            refused(&format!(r#"["workload-resolve",{B},"01"]"#)),
            ActionBodyRefusal::Delay
        );
        assert_eq!(
            refused(&format!(r#"["workload-resolve",{B},1]"#)),
            ActionBodyRefusal::Shape
        );
        assert_eq!(
            refused(&format!(r#"["workload-resolve",{B},"-1"]"#)),
            ActionBodyRefusal::Delay
        );
        assert_eq!(
            refused(r#"["identity",[{"op":"A"},null,[]]]"#),
            ActionBodyRefusal::Shape
        );
        assert_eq!(
            refused(r#"["post-state",["other",null,null,null,null,null]]"#),
            ActionBodyRefusal::Shape
        );
        // Pairs inside a row are in ascending schedule order, unique.
        assert_eq!(
            refused(&body(&format!(r#"[[[],[[{B},"1"],[{A},"1"]]]]"#))),
            ActionBodyRefusal::ScheduleOrder
        );
        assert_eq!(
            refused(&body(&format!(r#"[[[],[[{A},"1"],[{A},"2"]]]]"#))),
            ActionBodyRefusal::ScheduleOrder
        );
        assert_eq!(
            refused(&body(&format!(r#"[[[[{A},"1"]]]]"#))),
            ActionBodyRefusal::Shape
        );
    }
}
