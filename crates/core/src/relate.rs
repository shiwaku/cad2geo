//! 部分どうしの関係。同じ建物の階どうしは、共通する柱（円）で相対位置が決まる。
//!
//! 同じ半径の円の組から平行移動を仮定し、最も多くの円が一致する仮定を採る。そのあと一致した組で
//! 剛体変換（縮尺 1）を解き直す。図面の中の階どうしなので、縮尺は同じで回転はほぼ 0 とみなす。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Error;
use crate::fit::{self, Model, Pair, Similarity};
use crate::geojson::{bbox_center, xy};
use crate::part::{self, Part};

/// 関係を決めるのに要る、一致した円の最少の組数。
pub const MIN_PAIRS: usize = 3;
/// 同じ半径とみなす差（大きいほうの半径に対する比）。
const RADIUS_TOL: f64 = 0.05;
/// 解き直しの最大回数。
const MAX_ITER: usize = 10;

/// 図面座標の円。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: [f64; 2],
    pub radius: f64,
}

/// 一致した円の 1 組。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchedPair {
    /// この部分の円の中心（図面座標）
    pub from: [f64; 2],
    /// 相手の部分の円の中心（図面座標）
    pub to: [f64; 2],
    pub radius: f64,
    /// 相手の部分の中心 − 変換したこの部分の中心（図面の単位）
    pub residual: [f64; 2],
}

/// 部分から相手の部分への関係。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    /// 相手の部分
    pub to: String,
    /// この部分の図面座標 → 相手の部分の図面座標（縮尺 1）
    pub transform: Similarity,
    /// 残差の中央値と最大（図面の単位）
    pub residual_median: f64,
    pub residual_max: f64,
    /// 2 番目に多く一致した平行移動の組数。一致数に近いと取り違えの恐れがある
    pub runner_up: usize,
    pub pairs: Vec<MatchedPair>,
}

/// 図面座標の GeoJSON から円（`entity_type` が CIRCLE の閉じた折れ線）を取り出し、部分ごとに分ける。
/// 振り分けは位置合わせの適用と同じく、外接矩形の中心と先に書かれた部分の優先で行う。
pub fn circles_by_part(input: &Value, parts: &[Part]) -> Result<Vec<Vec<Circle>>, Error> {
    let features = input
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::GeoJson("input is not a FeatureCollection".into()))?;
    let mut out = vec![vec![]; parts.len()];
    for f in features {
        let props = f.get("properties");
        if props
            .and_then(|p| p.get("entity_type"))
            .and_then(Value::as_str)
            != Some("CIRCLE")
        {
            continue;
        }
        let Some(geom) = f.get("geometry").filter(|g| !g.is_null()) else {
            continue;
        };
        let Some(c) = circle(geom)? else {
            continue;
        };
        let layer = props.and_then(|p| p.get("layer")).and_then(Value::as_str);
        let Some(center) = bbox_center(geom) else {
            continue;
        };
        if let Some(i) = part::assign(parts, center, layer) {
            out[i].push(c);
        }
    }
    Ok(out)
}

/// 円を分割した折れ線から中心（頂点の平均）と半径（中心からの距離の平均）を求める。
fn circle(geom: &Value) -> Result<Option<Circle>, Error> {
    if geom.get("type").and_then(Value::as_str) != Some("LineString") {
        return Ok(None);
    }
    let Some(coords) = geom.get("coordinates").and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut pts = coords
        .iter()
        .map(|c| xy(c.as_array().map(Vec::as_slice).unwrap_or(&[])).map(|(x, y)| [x, y]))
        .collect::<Result<Vec<_>, _>>()?;
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < 3 {
        return Ok(None);
    }
    let n = pts.len() as f64;
    let center = [
        pts.iter().map(|p| p[0]).sum::<f64>() / n,
        pts.iter().map(|p| p[1]).sum::<f64>() / n,
    ];
    let radius = pts
        .iter()
        .map(|p| (p[0] - center[0]).hypot(p[1] - center[1]))
        .sum::<f64>()
        / n;
    Ok((radius > 0.0).then_some(Circle { center, radius }))
}

fn same_radius(a: &Circle, b: &Circle) -> bool {
    (a.radius - b.radius).abs() <= RADIUS_TOL * a.radius.max(b.radius)
}

/// `from` の円を `t` で写し、同じ半径で中心が半径以内にある `to` の円のうち最も近いものと組にする。
fn matches(from: &[Circle], to: &[Circle], t: &Similarity) -> Vec<(usize, usize)> {
    let mut out = vec![];
    for (i, a) in from.iter().enumerate() {
        let (x, y) = t.apply(a.center[0], a.center[1]);
        let mut best: Option<(f64, usize)> = None;
        for (j, b) in to.iter().enumerate() {
            if !same_radius(a, b) {
                continue;
            }
            let d = (b.center[0] - x).hypot(b.center[1] - y);
            if d <= a.radius.min(b.radius) && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, j));
            }
        }
        if let Some((_, j)) = best {
            out.push((i, j));
        }
    }
    out
}

fn shift(t: [f64; 2]) -> Similarity {
    Similarity {
        scale: 1.0,
        rotation_deg: 0.0,
        translation: t,
    }
}

/// `from` の部分から `to` の部分への関係を、共通する円で求める。
pub fn relate(from: &[Circle], to: &[Circle], to_name: &str) -> Result<Relation, Error> {
    // 平行移動の仮定ごとに一致数を数える
    let mut hyps: Vec<(usize, [f64; 2], f64)> = vec![];
    for a in from {
        for b in to.iter().filter(|b| same_radius(a, b)) {
            let t = [b.center[0] - a.center[0], b.center[1] - a.center[1]];
            hyps.push((matches(from, to, &shift(t)).len(), t, a.radius));
        }
    }
    let Some(&(_, best_t, best_r)) = hyps.iter().max_by_key(|h| h.0) else {
        return Err(Error::Relation(format!(
            "no circles of the same radius to match with part '{to_name}'"
        )));
    };
    // 最良の仮定から円の直径より離れた平行移動のうち、最も多く一致したもの
    let runner_up = hyps
        .iter()
        .filter(|h| (h.1[0] - best_t[0]).hypot(h.1[1] - best_t[1]) > 2.0 * best_r.max(h.2))
        .map(|h| h.0)
        .max()
        .unwrap_or(0);

    // 一致した組で剛体変換を解き直し、組が変わらなくなるまで繰り返す
    let mut t = shift(best_t);
    let mut ids = matches(from, to, &t);
    for _ in 0..MAX_ITER {
        if ids.len() < MIN_PAIRS {
            break;
        }
        let pairs: Vec<Pair> = ids
            .iter()
            .map(|&(i, j)| Pair {
                cad: from[i].center,
                map: to[j].center,
            })
            .collect();
        t = fit::solve(&pairs, Model::Rigid { scale: 1.0 })?;
        let next = matches(from, to, &t);
        if next == ids {
            break;
        }
        ids = next;
    }
    if ids.len() < MIN_PAIRS {
        return Err(Error::Relation(format!(
            "only {} circles match part '{to_name}' (need {MIN_PAIRS})",
            ids.len()
        )));
    }

    let pairs: Vec<MatchedPair> = ids
        .iter()
        .map(|&(i, j)| {
            let (x, y) = t.apply(from[i].center[0], from[i].center[1]);
            MatchedPair {
                from: from[i].center,
                to: to[j].center,
                radius: from[i].radius,
                residual: [to[j].center[0] - x, to[j].center[1] - y],
            }
        })
        .collect();
    let mut d: Vec<f64> = pairs
        .iter()
        .map(|p| p.residual[0].hypot(p.residual[1]))
        .collect();
    d.sort_by(f64::total_cmp);
    let m = d.len() / 2;
    let residual_median = if d.len() % 2 == 1 {
        d[m]
    } else {
        (d[m - 1] + d[m]) / 2.0
    };
    Ok(Relation {
        to: to_name.to_string(),
        transform: t,
        residual_median,
        residual_max: d[d.len() - 1],
        runner_up,
        pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 柱の格子（9 m 間隔）を、1 階と 2 階で少しずらして置いた図面。2 階は格子の一部だけ柱があり、
    /// 1 階に無い円（家具）もある。
    fn floors(rot_deg: f64, t: [f64; 2]) -> (Vec<Circle>, Vec<Circle>) {
        let rel = Similarity {
            scale: 1.0,
            rotation_deg: rot_deg,
            translation: t,
        };
        let mut f1 = vec![];
        let mut f2 = vec![];
        for i in 0..5 {
            for j in 0..4 {
                let c = [i as f64 * 9000.0, j as f64 * 9000.0];
                let r = if (i + j) % 3 == 0 { 300.0 } else { 450.0 };
                f1.push(Circle {
                    center: c,
                    radius: r,
                });
                // 2 階は東の 1 列が無い（格子 1 つ分ずらした仮定とは一致数が変わる）
                if i < 4 {
                    let (x, y) = rel.apply(c[0], c[1]);
                    // 作図の誤差（数十 mm）
                    let e = ((i * 7 + j * 3) % 5) as f64 * 10.0 - 20.0;
                    f2.push(Circle {
                        center: [x + e, y - e],
                        radius: r,
                    });
                }
            }
        }
        f1.push(Circle {
            center: [4500.0, 4500.0],
            radius: 25.0,
        });
        f2.push(Circle {
            center: [1234.0, 5678.0],
            radius: 25.0,
        });
        (f1, f2)
    }

    #[test]
    fn finds_offset_between_floors() {
        let (f1, f2) = floors(0.02, [-731.0, -134296.0]);
        let r = relate(&f1, &f2, "2F").unwrap();
        assert_eq!(r.pairs.len(), 16);
        assert!(r.runner_up < r.pairs.len(), "{}", r.runner_up);
        assert!((r.transform.rotation_deg - 0.02).abs() < 0.03);
        let (x, y) = r.transform.apply(18000.0, 13500.0);
        let (ex, ey) = Similarity {
            scale: 1.0,
            rotation_deg: 0.02,
            translation: [-731.0, -134296.0],
        }
        .apply(18000.0, 13500.0);
        assert!((x - ex).hypot(y - ey) < 30.0);
        assert!(r.residual_max <= 40.0 && r.residual_median <= r.residual_max);
    }

    #[test]
    fn too_few_common_circles() {
        let a = [Circle {
            center: [0.0, 0.0],
            radius: 450.0,
        }];
        let b = [Circle {
            center: [10.0, 0.0],
            radius: 450.0,
        }];
        assert!(matches!(relate(&a, &b, "2F"), Err(Error::Relation(_))));
        assert!(matches!(relate(&a, &[], "2F"), Err(Error::Relation(_))));
    }

    #[test]
    fn extracts_circles_per_part() {
        // 半径 1000 の円を 8 角形で近似したもの（閉じた折れ線）
        let ring = |cx: f64, cy: f64| {
            let mut v: Vec<Value> = (0..=8)
                .map(|k| {
                    let a = (k % 8) as f64 * std::f64::consts::FRAC_PI_4;
                    json!([cx + 1000.0 * a.cos(), cy + 1000.0 * a.sin()])
                })
                .collect();
            v.dedup();
            v
        };
        let input = json!({"type": "FeatureCollection", "features": [
            {"type": "Feature", "geometry": {"type": "LineString", "coordinates": ring(0.0, 0.0)},
             "properties": {"entity_type": "CIRCLE", "layer": "C"}},
            {"type": "Feature", "geometry": {"type": "LineString", "coordinates": ring(0.0, 50000.0)},
             "properties": {"entity_type": "CIRCLE", "layer": "C"}},
            {"type": "Feature", "geometry": {"type": "LineString", "coordinates": ring(0.0, 0.0)},
             "properties": {"entity_type": "ARC", "layer": "C"}}
        ]});
        let parts = vec![
            Part {
                name: "1F".into(),
                bbox: Some([-1e9, 10000.0, 1e9, 1e9]),
                ..Part::whole()
            },
            Part {
                name: "2F".into(),
                ..Part::whole()
            },
        ];
        let c = circles_by_part(&input, &parts).unwrap();
        assert_eq!((c[0].len(), c[1].len()), (1, 1));
        assert!((c[0][0].center[1] - 50000.0).abs() < 1e-9);
        assert!((c[1][0].radius - 1000.0).abs() < 1e-9);
    }
}
