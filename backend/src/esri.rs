//! XY-only EPSG:4326 conversion for the explicitly allowlisted normalized catalog.
//! Ring nesting determines shells/holes independently of Esri winding and order.
use serde_json::{Value, json};
type Point = [f64; 2];
type Ring = Vec<Point>;
fn point(x: &Value, y: &Value) -> Option<Point> {
    let p = [x.as_f64()?, y.as_f64()?];
    (p.iter().all(|n| n.is_finite())
        && (-180. ..=180.).contains(&p[0])
        && (-90. ..=90.).contains(&p[1]))
    .then_some(p)
}
fn area(r: &Ring) -> f64 {
    let [x, y] = r[0];
    r.windows(2)
        .map(|p| (p[0][0] - x) * (p[1][1] - y) - (p[1][0] - x) * (p[0][1] - y))
        .sum::<f64>()
        / 2.
}
// None means on boundary. A shared vertex must not determine ring ownership.
fn inside(p: Point, ring: &Ring) -> Option<bool> {
    let mut inside = false;
    for edge in ring.windows(2) {
        let [a, b] = [edge[0], edge[1]];
        let cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        if cross.abs() <= 1e-14
            && p[0] >= a[0].min(b[0])
            && p[0] <= a[0].max(b[0])
            && p[1] >= a[1].min(b[1])
            && p[1] <= a[1].max(b[1])
        {
            return None;
        }
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    Some(inside)
}
fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    fn side(a: Point, b: Point, p: Point) -> f64 {
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    }
    side(a, b, c) * side(a, b, d) < 0. && side(c, d, a) * side(c, d, b) < 0.
}
fn polygons(mut rings: Vec<Ring>) -> Option<Value> {
    for r in &mut rings {
        r.dedup();
        if r.first() != r.last() {
            r.push(*r.first()?);
        }
        if r.len() < 4 || area(r).abs() <= 1e-20 {
            return None;
        }
    }
    // Reject crossing boundaries (including self-crossings); never repair geometry
    // by dropping rings or treating every ring as a disconnected shell.
    for (i, ring) in rings.iter().enumerate() {
        for (j, other) in rings.iter().enumerate().skip(i) {
            for (a, edge) in ring.windows(2).enumerate() {
                for (b, second) in other.windows(2).enumerate() {
                    if i == j && b <= a {
                        continue;
                    }
                    if crosses(edge[0], edge[1], second[0], second[1]) {
                        return None;
                    }
                }
            }
        }
    }
    let areas: Vec<_> = rings.iter().map(|r| area(r).abs()).collect();
    let mut parents = vec![None; rings.len()];
    for (i, r) in rings.iter().enumerate() {
        for (j, outer) in rings.iter().enumerate() {
            if areas[j] <= areas[i] {
                continue;
            }
            if r.iter().find_map(|&p| inside(p, outer)) == Some(true)
                && parents[i].is_none_or(|k: usize| areas[j] < areas[k])
            {
                parents[i] = Some(j);
            }
        }
    }
    let depths: Vec<_> = (0..rings.len())
        .map(|i| {
            let mut depth = 0;
            let mut parent = parents[i];
            while let Some(p) = parent {
                depth += 1;
                parent = parents[p];
            }
            depth
        })
        .collect();
    let mut output = Vec::new();
    for (i, depth) in depths.iter().enumerate() {
        if depth % 2 != 0 {
            continue;
        }
        let mut polygon = Vec::new();
        for j in std::iter::once(i).chain((0..rings.len()).filter(|&j| parents[j] == Some(i))) {
            let mut r = rings[j].clone();
            if (area(&r) > 0.) != (j == i) {
                r.reverse();
            }
            polygon.push(r);
        }
        output.push(polygon);
    }
    if output.len() == 1 {
        Some(json!({"type":"Polygon","coordinates":output[0]}))
    } else {
        Some(json!({"type":"MultiPolygon","coordinates":output}))
    }
}
pub(crate) fn geometry(raw: &Value, kind: &str) -> Option<(Value, [f64; 4])> {
    if let Some(sr) = raw.get("spatialReference") {
        for key in ["wkid", "latestWkid"] {
            if sr.get(key).is_some_and(|v| v.as_i64() != Some(4326)) {
                return None;
            }
        }
        if sr.get("wkid").is_none() && sr.get("latestWkid").is_none() {
            return None;
        }
    }
    let mut bounds = [180_f64, 90_f64, -180_f64, -90_f64];
    let mut add = |p: Point| {
        bounds[0] = bounds[0].min(p[0]);
        bounds[1] = bounds[1].min(p[1]);
        bounds[2] = bounds[2].max(p[0]);
        bounds[3] = bounds[3].max(p[1]);
    };
    if kind == "point" {
        let p = point(raw.get("x")?, raw.get("y")?)?;
        add(p);
        return Some((json!({"type":"Point","coordinates":p}), bounds));
    }
    let field = if kind == "polygon" { "rings" } else { "paths" };
    let parts = raw.get(field)?.as_array()?;
    if parts.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    for part in parts {
        let points = part.as_array()?;
        if points.len() < 2 {
            return None;
        }
        let mut line = Vec::new();
        for p in points {
            let p = p.as_array()?;
            let p = point(p.first()?, p.get(1)?)?;
            add(p);
            line.push(p);
        }
        lines.push(line);
    }
    let geometry = if kind == "polygon" {
        polygons(lines)?
    } else if lines.len() == 1 {
        json!({"type":"LineString","coordinates":lines[0]})
    } else {
        json!({"type":"MultiLineString","coordinates":lines})
    };
    Some((geometry, bounds))
}
