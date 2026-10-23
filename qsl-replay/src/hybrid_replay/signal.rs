// SPDX-License-Identifier: AGPL-3.0-or-later
//! Three-valued truth signals over a behaviour's positions and the
//! pointwise meaning of a claim's operators over them (QSpec FR-193
//! "Behaviours and positions", FR-416 "Pointwise meaning").
//!
//! A timeline is a sequence of segments, each a stretch of positions with
//! clock values `[low, high]` (`[low, high)` for a cut segment), ordered by
//! segment and then by clock, followed by an unknown tail: every position
//! past the last known one, with clock values from `tail` on. A signal gives
//! each segment a partition into instants and open stretches, each with one
//! truth value; every tail position is unknown.

use std::cmp::Ordering;

use quire_exact::Rational;

use super::interval::Tri;
use super::real::ExactReal;

/// The clock span of one segment.
#[derive(Clone, Debug)]
pub(crate) struct Span {
    pub(crate) low: ExactReal,
    pub(crate) high: ExactReal,
    pub(crate) high_closed: bool,
}

/// One stretch of a segment's partition: the instant `low` when `point`,
/// else the open interval `(low, high)`.
#[derive(Clone, Debug)]
pub(crate) struct Piece {
    pub(crate) low: ExactReal,
    pub(crate) high: ExactReal,
    pub(crate) point: bool,
    pub(crate) value: Tri,
}

/// A truth value per position of a timeline's known segments.
pub(crate) type Signal = Vec<Vec<Piece>>;

/// An operator's interval over clock offsets.
#[derive(Clone, Debug)]
pub(crate) struct Window {
    pub(crate) lower: Rational,
    pub(crate) lower_open: bool,
    pub(crate) upper: Option<Rational>,
    pub(crate) upper_open: bool,
}

/// One end of a stretch of clock values.
#[derive(Clone, Debug)]
struct End {
    at: ExactReal,
    open: bool,
}

/// Whether the clock values between `lower` and `upper` (`None`: no end)
/// are a nonempty set.
fn nonempty(lower: Option<&End>, upper: Option<&End>) -> bool {
    match (lower, upper) {
        (None, _) | (_, None) => true,
        (Some(lower), Some(upper)) => match lower.at.cmp(&upper.at) {
            Ordering::Less => true,
            Ordering::Equal => !lower.open && !upper.open,
            Ordering::Greater => false,
        },
    }
}

/// The greater of two lower ends, `None` being no end; at a tie, open when
/// either is.
fn max_lower(a: Option<End>, b: Option<End>) -> Option<End> {
    match (a, b) {
        (None, other) | (other, None) => other,
        (Some(a), Some(b)) => Some(match a.at.cmp(&b.at) {
            Ordering::Greater => a,
            Ordering::Less => b,
            Ordering::Equal => End {
                open: a.open || b.open,
                at: a.at,
            },
        }),
    }
}

/// The lesser of two upper ends, `None` being no end.
fn min_upper(a: Option<End>, b: Option<End>) -> Option<End> {
    match (a, b) {
        (None, other) | (other, None) => other,
        (Some(a), Some(b)) => Some(match a.at.cmp(&b.at) {
            Ordering::Less => a,
            Ordering::Greater => b,
            Ordering::Equal => End {
                open: a.open || b.open,
                at: a.at,
            },
        }),
    }
}

/// The pieces of `span` cut at `breakpoints`.
fn partition(span: &Span, breakpoints: Vec<ExactReal>) -> Vec<(ExactReal, ExactReal, bool)> {
    let mut points: Vec<ExactReal> = breakpoints
        .into_iter()
        .filter(|b| *b >= span.low && (*b < span.high || (span.high_closed && *b == span.high)))
        .collect();
    points.push(span.low.clone());
    if span.high_closed {
        points.push(span.high.clone());
    }
    points.sort();
    points.dedup();
    let mut pieces = Vec::with_capacity(points.len() * 2);
    for (i, point) in points.iter().enumerate() {
        pieces.push((point.clone(), point.clone(), true));
        let next = points
            .get(i + 1)
            .cloned()
            .or_else(|| (!span.high_closed && *point < span.high).then(|| span.high.clone()));
        if let Some(next) = next {
            pieces.push((point.clone(), next, false));
        }
    }
    pieces
}

/// The value of `pieces` at the instant `x`.
fn value_at(pieces: &[Piece], x: &ExactReal) -> Tri {
    pieces
        .iter()
        .find(|piece| {
            if piece.point {
                piece.low == *x
            } else {
                piece.low < *x && *x < piece.high
            }
        })
        .map_or(Tri::Unknown, |piece| piece.value)
}

/// A sample instant of the stretch `(low, high, point)`.
fn sample(low: &ExactReal, high: &ExactReal, point: bool) -> ExactReal {
    if point {
        low.clone()
    } else {
        ExactReal::Rational(low.between(high))
    }
}

/// Every breakpoint of `signal`'s segment `k`, the segment's ends included.
fn breakpoints(signal: &Signal, timeline: &Timeline, k: usize) -> Vec<ExactReal> {
    let mut points: Vec<ExactReal> = signal
        .get(k)
        .map(|pieces| {
            pieces
                .iter()
                .filter(|p| p.point)
                .map(|p| p.low.clone())
                .collect()
        })
        .unwrap_or_default();
    if let Some(span) = timeline.spans.get(k) {
        points.push(span.low.clone());
        points.push(span.high.clone());
    }
    points
}

/// The positions of a behaviour a signal is over.
#[derive(Clone, Debug)]
pub(crate) struct Timeline {
    pub(crate) spans: Vec<Span>,
    /// Every position past the known ones has a clock value at or after
    /// this; `None` when the timeline has no unknown tail.
    pub(crate) tail: Option<ExactReal>,
}

/// A position's direction of reading for a window operator.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Direction {
    Future,
    Past,
}

/// One stretch of positions as an operator reads it from a position.
struct Stretch {
    low: ExactReal,
    high: ExactReal,
    point: bool,
    values: [Tri; 2],
}

impl Timeline {
    /// `f` of two signals, piece by piece.
    pub(crate) fn combine(&self, a: &Signal, b: &Signal, f: impl Fn(Tri, Tri) -> Tri) -> Signal {
        self.spans
            .iter()
            .enumerate()
            .map(|(k, span)| {
                let empty = Vec::new();
                let (pa, pb) = (a.get(k).unwrap_or(&empty), b.get(k).unwrap_or(&empty));
                let mut points = breakpoints(a, self, k);
                points.extend(breakpoints(b, self, k));
                partition(span, points)
                    .into_iter()
                    .map(|(low, high, point)| {
                        let at = sample(&low, &high, point);
                        Piece {
                            value: f(value_at(pa, &at), value_at(pb, &at)),
                            low,
                            high,
                            point,
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// `f` of one signal, piece by piece.
    pub(crate) fn map(signal: &Signal, f: impl Fn(Tri) -> Tri) -> Signal {
        signal
            .iter()
            .map(|pieces| {
                pieces
                    .iter()
                    .map(|piece| Piece {
                        value: f(piece.value),
                        ..piece.clone()
                    })
                    .collect()
            })
            .collect()
    }

    /// The pieces of `a` and `b` merged on segment `k`.
    fn merged(&self, a: &Signal, b: &Signal, k: usize) -> Vec<Stretch> {
        let empty = Vec::new();
        let (pa, pb) = (a.get(k).unwrap_or(&empty), b.get(k).unwrap_or(&empty));
        let Some(span) = self.spans.get(k) else {
            return Vec::new();
        };
        let mut points = breakpoints(a, self, k);
        points.extend(breakpoints(b, self, k));
        partition(span, points)
            .into_iter()
            .map(|(low, high, point)| {
                let at = sample(&low, &high, point);
                Stretch {
                    values: [value_at(pa, &at), value_at(pb, &at)],
                    low,
                    high,
                    point,
                }
            })
            .collect()
    }

    /// The stretches a `direction` operator at `(k, s)` reads, in reading
    /// order, starting with the position itself, then the tail's start when
    /// reading the future.
    fn reading(
        &self,
        merged: &[Vec<Stretch>],
        k: usize,
        s: &ExactReal,
        direction: Direction,
    ) -> Vec<Stretch> {
        let mut out = Vec::new();
        let own = merged.get(k).map(Vec::as_slice).unwrap_or(&[]);
        let split = |stretch: &Stretch, out: &mut Vec<Stretch>| {
            if stretch.point {
                let keep = match direction {
                    Direction::Future => stretch.low >= *s,
                    Direction::Past => stretch.low <= *s,
                };
                if keep {
                    out.push(Stretch {
                        low: stretch.low.clone(),
                        high: stretch.high.clone(),
                        point: true,
                        values: stretch.values,
                    });
                }
                return;
            }
            let inside = stretch.low < *s && *s < stretch.high;
            let (from, to) = match direction {
                Direction::Future if inside => (s.clone(), stretch.high.clone()),
                Direction::Past if inside => (stretch.low.clone(), s.clone()),
                Direction::Future if stretch.low >= *s => {
                    (stretch.low.clone(), stretch.high.clone())
                }
                Direction::Past if stretch.high <= *s => {
                    (stretch.low.clone(), stretch.high.clone())
                }
                Direction::Future | Direction::Past => return,
            };
            if inside {
                out.push(Stretch {
                    low: s.clone(),
                    high: s.clone(),
                    point: true,
                    values: stretch.values,
                });
            }
            out.push(Stretch {
                low: from,
                high: to,
                point: false,
                values: stretch.values,
            });
        };
        match direction {
            Direction::Future => {
                for stretch in own {
                    split(stretch, &mut out);
                }
                for segment in merged.iter().skip(k + 1) {
                    for stretch in segment {
                        out.push(Stretch {
                            low: stretch.low.clone(),
                            high: stretch.high.clone(),
                            point: stretch.point,
                            values: stretch.values,
                        });
                    }
                }
            }
            Direction::Past => {
                for stretch in own.iter().rev() {
                    split(stretch, &mut out);
                }
                for segment in merged.iter().take(k).rev() {
                    for stretch in segment.iter().rev() {
                        out.push(Stretch {
                            low: stretch.low.clone(),
                            high: stretch.high.clone(),
                            point: stretch.point,
                            values: stretch.values,
                        });
                    }
                }
            }
        }
        out
    }

    /// The clock values a `direction` operator with `window` reads from
    /// `s`: its lower and upper end.
    fn reach(window: &Window, s: &ExactReal, direction: Direction) -> (Option<End>, Option<End>) {
        match direction {
            Direction::Future => (
                Some(End {
                    at: s.shifted(&window.lower),
                    open: window.lower_open,
                }),
                window.upper.as_ref().map(|upper| End {
                    at: s.shifted(upper),
                    open: window.upper_open,
                }),
            ),
            Direction::Past => (
                window.upper.as_ref().map(|upper| End {
                    at: s.shifted(&upper.neg()),
                    open: window.upper_open,
                }),
                Some(End {
                    at: s.shifted(&window.lower.neg()),
                    open: window.lower_open,
                }),
            ),
        }
    }

    /// Whether the stretch meets the clock values `[lower, upper]`.
    fn meets(stretch: &Stretch, lower: Option<&End>, upper: Option<&End>) -> bool {
        let own_lower = End {
            at: stretch.low.clone(),
            open: !stretch.point,
        };
        let own_upper = End {
            at: stretch.high.clone(),
            open: !stretch.point,
        };
        let low = max_lower(Some(own_lower), lower.cloned());
        let high = min_upper(Some(own_upper), upper.cloned());
        nonempty(low.as_ref(), high.as_ref())
    }

    /// The candidate breakpoints of an operator's output on segment `k`.
    fn candidates(
        &self,
        signals: &[&Signal],
        window: &Window,
        direction: Direction,
    ) -> Vec<ExactReal> {
        let mut points = Vec::new();
        for k in 0..self.spans.len() {
            for signal in signals {
                points.extend(breakpoints(signal, self, k));
            }
        }
        if let Some(tail) = &self.tail {
            points.push(tail.clone());
        }
        let mut shifted = Vec::with_capacity(points.len() * 3);
        for point in &points {
            shifted.push(point.clone());
            let mut offsets = vec![window.lower.clone()];
            offsets.extend(window.upper.clone());
            for offset in offsets {
                let offset = match direction {
                    Direction::Future => offset.neg(),
                    Direction::Past => offset,
                };
                shifted.push(point.shifted(&offset));
            }
        }
        shifted
    }

    /// The output of a two-operand operator, evaluated at a sample of each
    /// stretch of its candidate partition.
    fn operator(
        &self,
        a: &Signal,
        b: &Signal,
        window: &Window,
        direction: Direction,
        at: impl Fn(&[Stretch], Option<&End>, Option<&End>, bool) -> Tri,
    ) -> Signal {
        let merged: Vec<Vec<Stretch>> = (0..self.spans.len())
            .map(|k| self.merged(a, b, k))
            .collect();
        let candidates = self.candidates(&[a, b], window, direction);
        self.spans
            .iter()
            .enumerate()
            .map(|(k, span)| {
                partition(span, candidates.clone())
                    .into_iter()
                    .map(|(low, high, point)| {
                        let s = sample(&low, &high, point);
                        let reading = self.reading(&merged, k, &s, direction);
                        let (lower, upper) = Self::reach(window, &s, direction);
                        let tail = direction == Direction::Future
                            && self.tail.as_ref().is_some_and(|tail| {
                                let tail = End {
                                    at: tail.clone(),
                                    open: false,
                                };
                                nonempty(
                                    max_lower(Some(tail), lower.clone()).as_ref(),
                                    upper.as_ref(),
                                )
                            });
                        Piece {
                            value: at(&reading, lower.as_ref(), upper.as_ref(), tail),
                            low,
                            high,
                            point,
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// `eventually` (`always` when `universal`) with `window`, reading the
    /// future, or `once` (`historically`) reading the past.
    pub(crate) fn unary(&self, f: &Signal, window: &Window, past: bool, universal: bool) -> Signal {
        let direction = if past {
            Direction::Past
        } else {
            Direction::Future
        };
        self.operator(f, f, window, direction, |reading, lower, upper, tail| {
            let mut value = Tri::from_bool(universal);
            for stretch in reading {
                if Self::meets(stretch, lower, upper) {
                    value = if universal {
                        value.and(stretch.values[0])
                    } else {
                        value.or(stretch.values[0])
                    };
                }
            }
            if tail {
                value = if universal {
                    value.and(Tri::Unknown)
                } else {
                    value.or(Tri::Unknown)
                };
            }
            value
        })
    }

    /// `p until q` with `window` reading the future, or `p since q` reading
    /// the past: some position `j` in the window has `q`, and every position
    /// from the reading position up to `j`, `j` excluded, has `p`.
    pub(crate) fn binary(&self, p: &Signal, q: &Signal, window: &Window, past: bool) -> Signal {
        let direction = if past {
            Direction::Past
        } else {
            Direction::Future
        };
        self.operator(p, q, window, direction, |reading, lower, upper, tail| {
            let mut value = Tri::False;
            let mut prefix = Tri::True;
            for stretch in reading {
                let [p, q] = stretch.values;
                if Self::meets(stretch, lower, upper) {
                    let here = if stretch.point {
                        q.and(prefix)
                    } else {
                        q.and(prefix).and(p)
                    };
                    value = value.or(here);
                }
                prefix = prefix.and(p);
            }
            if tail {
                value = value.or(Tri::Unknown.and(prefix));
            }
            value
        })
    }

    /// The value of `signal` at the first position.
    pub(crate) fn at_origin(&self, signal: &Signal) -> Tri {
        match (signal.first(), self.spans.first()) {
            (Some(pieces), Some(span)) => value_at(pieces, &span.low),
            _ => Tri::Unknown,
        }
    }

    /// The value of `signal` at the instant `at` of segment `k`.
    pub(crate) fn value(signal: &Signal, k: usize, at: &ExactReal) -> Tri {
        signal
            .get(k)
            .map_or(Tri::Unknown, |pieces| value_at(pieces, at))
    }
}
