//! Shapes: rectangles (with a radius per corner from their points), ovals, polygons, stars,
//! triangles, lines and vector paths from `Vertex` points. A shape whose points were edited, or
//! don't trace its regular outline, is imported as a vector path.
//!
//! A `Vertex` is `[x, y, mode, radius, fromX, fromY, toX, toY]` in the layer's unit box (the
//! README's rectangle of 431 × 428 has the points `[[0,0],[1,0],[1,1],[0,1]]`). `from` is the
//! control point leaving the point and `to` the one arriving at it; which modes carry which
//! handles (`OnlyFrom`, `OnlyTo`, the rest both) follows the schema's `Vertex` (`src/cs`, read to
//! settle what the README's `CurveMode` names leave open).

use figcraft_doc::geom::Point;
use figcraft_doc::{HandleMirroring, Node, NodeKind, Region, Segment, VectorNetwork, Vertex, WindingRule};
use serde_json::Value;

use super::Ctx;
use super::json::{self, MAX_SIZE, flag, int, list, num};
use crate::import_report::Severity;

/// Most points read from one shape.
const MAX_POINTS: usize = 100_000;
/// How close (in the unit box) a point must be to the regular outline to keep a polygon or star.
const MATCH: f64 = 1e-3;

/// One `Vertex`, in the layer's own units.
struct VertexPoint {
    at: Point,
    from: Option<Point>,
    to: Option<Point>,
    radius: f64,
    mode: i64,
}

/// The `points` of `j`, scaled from the unit box to `w` × `h`; None when one can't be read.
fn vertices(j: &Value, w: f64, h: f64) -> Option<Vec<VertexPoint>> {
    let scale = |x: f64, y: f64| Point::new(x.clamp(-1e4, 1e4) * w, y.clamp(-1e4, 1e4) * h);
    let mut out = Vec::new();
    for p in list(j, "points").iter().take(MAX_POINTS) {
        let v = json::floats(p, 8)?;
        let at = |i: usize| v.get(i).copied().unwrap_or(0.0);
        if !matches!(v.len(), 2 | 3 | 4 | 6 | 8) {
            return None;
        }
        let mode = at(2) as i64;
        let has_from = matches!(mode, 1..=4);
        let has_to = matches!(mode, 1..=3 | 5);
        out.push(VertexPoint {
            at: scale(at(0), at(1)),
            from: has_from.then(|| scale(at(4), at(5))),
            to: has_to.then(|| scale(at(6), at(7))),
            radius: at(3).clamp(0.0, MAX_SIZE),
            mode,
        });
    }
    Some(out)
}

/// The points' positions in the unit box, without curves (None when any point curves).
fn straight_unit(points: &[VertexPoint], w: f64, h: f64) -> Option<Vec<Point>> {
    if w <= 0.0 || h <= 0.0 || points.iter().any(|p| p.from.is_some() || p.to.is_some()) {
        return None;
    }
    Some(points.iter().map(|p| Point::new(p.at.x / w, p.at.y / h)).collect())
}

/// Whether `got` is `want` (both in the unit box), starting at any of `want`'s points.
fn same_outline(got: &[Point], want: &[Point]) -> bool {
    let n = want.len();
    n > 0
        && got.len() == n
        && (0..n).any(|shift| got.iter().enumerate().all(|(i, p)| want.get((i + shift) % n).is_some_and(|q| (*p - *q).hypot() <= MATCH)))
}

/// The vector network through `points`, closing back to the first when `closed`.
fn network(points: &[VertexPoint], closed: bool, winding: WindingRule) -> Option<VectorNetwork> {
    let n = points.len();
    if n < 2 {
        return None;
    }
    let vertices = points
        .iter()
        .map(|p| Vertex {
            radius: p.radius,
            mirror: match p.mode {
                1 => Some(HandleMirroring::AngleAndLength),
                2 => Some(HandleMirroring::Angle),
                3..=5 => Some(HandleMirroring::None),
                _ => None,
            },
            ..Vertex::new(p.at.x, p.at.y)
        })
        .collect();
    let count = if closed { n } else { n - 1 };
    let mut segments = Vec::with_capacity(count);
    for i in 0..count {
        let b = (i + 1) % n;
        let (Some(pa), Some(pb)) = (points.get(i), points.get(b)) else { continue };
        let ta = pa.from.map_or((0.0, 0.0), |c| (c.x - pa.at.x, c.y - pa.at.y));
        let tb = pb.to.map_or((0.0, 0.0), |c| (c.x - pb.at.x, c.y - pb.at.y));
        segments.push(Segment { a: i, b, ta, tb });
    }
    let regions = if closed { vec![Region { loops: vec![(0..segments.len()).collect()], winding }] } else { Vec::new() };
    let mut net = VectorNetwork { vertices, segments, regions };
    net.sanitize();
    Some(net)
}

impl Ctx<'_> {
    /// Set `n`'s kind (and corners) from the shape layer `j` of type `t`.
    pub(super) fn shape(&mut self, n: &mut Node, t: &str, j: &Value, path: &str) {
        let (w, h) = (n.width, n.height);
        let points = vertices(j, w, h).unwrap_or_else(|| {
            self.warn(Severity::Dropped, path, "The shape's points couldn't be read; it was imported without them.");
            Vec::new()
        });
        let edited = flag(j, "edited") == Some(true);
        if j.get("pathData").is_some() {
            let what = if points.is_empty() { "the shape has no outline here" } else { "its points were used" };
            self.warn(
                Severity::Dropped,
                path,
                format!("The shape's SVG path data was not read (the README doesn't give its coordinate space); {what}."),
            );
        }
        let regular = match t {
            "RECT" if !edited => {
                for p in &points {
                    let (left, top) = (p.at.x < w / 2.0, p.at.y < h / 2.0);
                    let corner = match (left, top) {
                        (true, true) => 0,
                        (false, true) => 1,
                        (false, false) => 2,
                        (true, false) => 3,
                    };
                    if let Some(r) = n.corners.radii.get_mut(corner).filter(|_| p.radius > 0.0) {
                        *r = p.radius;
                    }
                }
                Some(NodeKind::Rectangle)
            }
            "OVAL" if !edited => {
                let arc = ["angle", "startAngle", "ratio"].iter().any(|k| num(j, k).is_some_and(|x| x != 0.0));
                if arc {
                    self.warn(
                        Severity::Approximated,
                        path,
                        "The oval's arc and donut settings were not imported (the README doesn't give their units); it is a full ellipse.",
                    );
                }
                Some(NodeKind::Ellipse { start: 0.0, sweep: std::f64::consts::TAU, inner: 0.0 })
            }
            "POLYGON" | "STAR" | "TRIANGLE" if !edited => self.regular(n, t, j, &points, path),
            _ => None,
        };
        if let Some(kind) = regular {
            n.kind = kind;
            return;
        }
        let closed = t != "LINE" && flag(j, "open") != Some(true);
        let winding = if int(j, "winding") == Some(0) { WindingRule::NonZero } else { WindingRule::EvenOdd };
        match network(&points, closed, winding) {
            Some(network) => n.kind = NodeKind::Vector { network },
            None => {
                self.warn(Severity::Dropped, path, "The shape has fewer than two points; it was imported as an empty path.");
                n.kind = NodeKind::Vector { network: VectorNetwork::default() };
            }
        }
    }

    /// A polygon, star or triangle as its regular kind: from `rays` / `ratio` when it has no
    /// points, or when its points trace that outline; None otherwise (it is then a vector path).
    fn regular(&mut self, n: &mut Node, t: &str, j: &Value, points: &[VertexPoint], path: &str) -> Option<NodeKind> {
        let count = match t {
            "TRIANGLE" => 3,
            _ => match num(j, "rays").filter(|r| *r >= 3.0) {
                Some(r) => r.round().min(120.0) as u32,
                None if points.len() >= 2 => return None,
                None => {
                    self.warn(Severity::Approximated, path, "The shape gives fewer than 3 corners and no points; it was imported with 5.");
                    5
                }
            },
        };
        let ratio = num(j, "ratio").filter(|r| *r > 0.0).map(|r| r.clamp(0.001, 1.0));
        let kind = match (t, ratio) {
            ("STAR", Some(inner_ratio)) => NodeKind::Star { count, inner_ratio },
            ("STAR", None) if points.len() >= 2 => return None,
            ("STAR", None) => {
                self.warn(Severity::Approximated, path, "The star has no ratio and no points; it was imported with a ratio of 0.5.");
                NodeKind::Star { count, inner_ratio: 0.5 }
            }
            _ => NodeKind::Polygon { count },
        };
        if points.is_empty() {
            return Some(kind);
        }
        let want = match &kind {
            NodeKind::Star { count, inner_ratio } => figcraft_doc::geom::star_points(1.0, 1.0, *count, *inner_ratio),
            _ => figcraft_doc::geom::polygon_points(1.0, 1.0, count),
        };
        let got = straight_unit(points, n.width, n.height)?;
        if !same_outline(&got, &want) {
            self.warn(Severity::Info, path, format!("The {} was imported as a vector path to keep its exact outline.", t.to_lowercase()));
            return None;
        }
        let radius = points.first().map_or(0.0, |p| p.radius);
        if points.iter().any(|p| (p.radius - radius).abs() > 1e-9) {
            self.warn(Severity::Approximated, path, "Points with different corner radii were imported with the first point's radius.");
        }
        n.corners.radii = [radius; 4];
        Some(kind)
    }
}
