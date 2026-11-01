//! Shapes: rectangles (with a radius per corner), ovals, polygons, stars, triangles and vector
//! paths from curve points. A shape whose points were edited is imported as a vector path.

use figcraft_doc::geom::Point;
use figcraft_doc::{HandleMirroring, Node, NodeKind, Region, Segment, VectorNetwork, Vertex, WindingRule};
use serde_json::Value;

use super::Ctx;
use super::json::{MAX_SIZE, flag, int, list, num, point_at};
use crate::import_report::Severity;

/// Most curve points read from one shape.
const MAX_POINTS: usize = 100_000;
/// How close (in the unit box) a point must be to the regular outline to keep a polygon or star.
const MATCH: f64 = 1e-3;
/// The corner smoothing Sketch's "smooth" corners are imported with.
const SMOOTH: f64 = 0.6;

/// One curve point, in the layer's own units.
struct CurvePoint {
    at: Point,
    /// The control point leaving it (when it curves out).
    from: Option<Point>,
    /// The control point arriving at it (when it curves in).
    to: Option<Point>,
    radius: f64,
    mode: Option<i64>,
    style: i64,
}

/// The `points` of `j`, scaled from the unit box to `w` × `h`.
fn curve_points(j: &Value, w: f64, h: f64) -> Vec<CurvePoint> {
    let scale = |(x, y): (f64, f64)| Point::new(x * w, y * h);
    list(j, "points")
        .iter()
        .take(MAX_POINTS)
        .filter_map(|p| {
            let at = scale(point_at(p, "point")?);
            let mode = int(p, "curveMode");
            // Straight points have no handles, whatever they store.
            let curved = mode != Some(1);
            let from = (curved && flag(p, "hasCurveFrom") == Some(true)).then(|| point_at(p, "curveFrom")).flatten().map(scale);
            let to = (curved && flag(p, "hasCurveTo") == Some(true)).then(|| point_at(p, "curveTo")).flatten().map(scale);
            let radius = num(p, "cornerRadius").unwrap_or(0.0).clamp(0.0, MAX_SIZE);
            Some(CurvePoint { at, from, to, radius, mode, style: int(p, "cornerStyle").unwrap_or(0) })
        })
        .collect()
}

/// The points' positions in the unit box, without curves (None when any point curves).
fn straight_unit(points: &[CurvePoint], w: f64, h: f64) -> Option<Vec<Point>> {
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

/// The vector network through `points`: segment `i` runs from point `i` to the next, closing
/// back to the first when `closed`.
fn network(points: &[CurvePoint], closed: bool, winding: WindingRule, rounded: bool) -> Option<VectorNetwork> {
    let n = points.len();
    if n < 2 {
        return None;
    }
    let vertices = points
        .iter()
        .map(|p| Vertex {
            radius: if rounded { p.radius } else { 0.0 },
            mirror: match p.mode {
                Some(2) => Some(HandleMirroring::AngleAndLength),
                Some(3) => Some(HandleMirroring::Angle),
                Some(1 | 4) => Some(HandleMirroring::None),
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
    /// Set `n`'s kind (and corners) from the shape layer `j` of `class`.
    pub(super) fn shape(&mut self, n: &mut Node, class: &str, j: &Value, path: &str) {
        let (w, h) = (n.width, n.height);
        let points = curve_points(j, w, h);
        let edited = flag(j, "edited") == Some(true);
        let rounded = int(j, "pointRadiusBehaviour") != Some(-1);
        let smooth = int(j, "pointRadiusBehaviour") == Some(2);
        if rounded && points.iter().any(|p| p.radius > 0.0 && p.style != 0) {
            self.warn(Severity::Approximated, path, "Inverted, angled or squared corners were imported as rounded corners.");
        }
        let regular = match class {
            "rectangle" if !edited => {
                let mut radii = [0.0; 4];
                for p in &points {
                    let (left, top) = (p.at.x < w / 2.0, p.at.y < h / 2.0);
                    let corner = match (left, top) {
                        (true, true) => 0,
                        (false, true) => 1,
                        (false, false) => 2,
                        (true, false) => 3,
                    };
                    if let Some(r) = radii.get_mut(corner) {
                        *r = p.radius;
                    }
                }
                if radii.iter().all(|r| *r == 0.0)
                    && let Some(f) = num(j, "fixedRadius").filter(|r| *r > 0.0)
                {
                    radii = [f.min(MAX_SIZE); 4];
                }
                if !rounded {
                    radii = [0.0; 4];
                }
                n.corners.radii = radii;
                Some(NodeKind::Rectangle)
            }
            "oval" if !edited => Some(NodeKind::Ellipse { start: 0.0, sweep: std::f64::consts::TAU, inner: 0.0 }),
            "polygon" | "star" | "triangle" if !edited => self.regular(class, j, &points, w, h, path).map(|(kind, radius)| {
                n.corners.radii = [if rounded { radius } else { 0.0 }; 4];
                kind
            }),
            _ => None,
        };
        if let Some(kind) = regular {
            if smooth && n.corners.radii.iter().any(|r| *r > 0.0) {
                n.corners.smoothing = SMOOTH;
                self.warn(Severity::Approximated, path, "Smooth corners were imported with 60% corner smoothing.");
            }
            n.kind = kind;
            return;
        }
        let closed = flag(j, "isClosed").unwrap_or(true);
        let winding = match j.get("style").and_then(|s| int(s, "windingRule")) {
            Some(1) => WindingRule::EvenOdd,
            _ => WindingRule::NonZero,
        };
        match network(&points, closed, winding, rounded) {
            Some(network) => n.kind = NodeKind::Vector { network },
            None => {
                self.warn(Severity::Dropped, path, "The shape has fewer than two points; it was imported as an empty path.");
                n.kind = NodeKind::Vector { network: VectorNetwork::default() };
            }
        }
        if !closed && n.fills.iter().any(|p| p.visible) {
            self.warn(Severity::Approximated, path, "Sketch fills open paths as if closed; here the open path shows no fill.");
        }
        if smooth && rounded && points.iter().any(|p| p.radius > 0.0) {
            self.warn(Severity::Approximated, path, "Smooth point corners were imported as round corners.");
        }
    }

    /// A polygon, star or triangle whose points are the regular outline of its kind; None when
    /// they aren't (the shape is then imported as a vector path, keeping its exact outline).
    fn regular(&mut self, class: &str, j: &Value, points: &[CurvePoint], w: f64, h: f64, path: &str) -> Option<(NodeKind, f64)> {
        let got = straight_unit(points, w, h)?;
        let count = if class == "triangle" { 3 } else { int(j, "numberOfPoints")?.clamp(3, 120) as u32 };
        let (want, kind) = if class == "star" {
            let ratio = num(j, "radius").unwrap_or(0.5).clamp(0.001, 1.0);
            (figcraft_doc::geom::star_points(1.0, 1.0, count, ratio), NodeKind::Star { count, inner_ratio: ratio })
        } else {
            (figcraft_doc::geom::polygon_points(1.0, 1.0, count), NodeKind::Polygon { count })
        };
        if !same_outline(&got, &want) {
            if class != "triangle" {
                self.warn(Severity::Info, path, format!("The {class} was imported as a vector path to keep its exact outline."));
            }
            return None;
        }
        let radius = points.first().map_or(0.0, |p| p.radius);
        if points.iter().any(|p| (p.radius - radius).abs() > 1e-9) {
            self.warn(Severity::Approximated, path, "Points with different corner radii were imported with the first point's radius.");
        }
        Some((kind, radius))
    }
}
