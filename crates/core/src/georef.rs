//! 部分ごとの位置合わせと、その記録（JSON）。
//!
//! 記録を入力に渡せば同じ変換を再現できる（対応点を解き直さない）。

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::crs::{Crs, Transformer};
use crate::fit::{self, ErrorStats, Model, Pair, Similarity};
use crate::part::Part;

pub const RECORD_VERSION: u32 = 1;

/// 対応点の役割。検証点は変換を解くのに使わず、誤差を測るだけに使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    #[default]
    Control,
    Check,
}

/// 対応点。`map` は `gcp_crs` の座標（経緯度なら x = 経度）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gcp {
    pub part: String,
    pub cad: [f64; 2],
    pub map: [f64; 2],
    #[serde(default)]
    pub role: Role,
}

/// 縮尺の扱い。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScaleMode {
    /// 単位から決めた縮尺（図面の 1 単位が何 m か）に固定する
    Fixed(f64),
    /// 相似変換で縮尺も解く
    Free,
}

/// 位置合わせの入力。
#[derive(Debug, Clone)]
pub struct Request {
    pub parts: Vec<Part>,
    pub gcps: Vec<Gcp>,
    pub gcp_crs: Crs,
    pub work_crs: Crs,
    pub scale: ScaleMode,
}

/// 対応点 1 つの結果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointResult {
    pub cad: [f64; 2],
    pub map: [f64; 2],
    /// 作業用の投影座標系での地図側の座標
    pub map_work: [f64; 2],
    /// 作業用の投影座標系での誤差（地図側 − 変換した図面側、m）
    pub residual: [f64; 2],
}

/// 部分 1 つの位置合わせの結果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartGeoref {
    #[serde(flatten)]
    pub part: Part,
    /// 載せない部分には無い
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<Similarity>,
    /// 縮尺を固定したとき、相似変換で解いた場合の縮尺（単位の誤りや対応点の偏りに気づくため）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub free_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control: Vec<PointResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub check: Vec<PointResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_error: Option<ErrorStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_error: Option<ErrorStats>,
}

/// 位置合わせの記録。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub cad2geojson_georef: u32,
    /// 作業用の投影座標系（"EPSG:6673" など）
    pub work_crs: String,
    pub gcp_crs: String,
    pub parts: Vec<PartGeoref>,
}

impl Record {
    pub fn work_crs(&self) -> Result<Crs, Error> {
        self.work_crs.parse()
    }

    pub fn from_json(s: &str) -> Result<Record, Error> {
        let r: Record = serde_json::from_str(s).map_err(|e| Error::Record(e.to_string()))?;
        if r.cad2geojson_georef != RECORD_VERSION {
            return Err(Error::Record(format!(
                "unsupported record version {}",
                r.cad2geojson_georef
            )));
        }
        for p in &r.parts {
            if p.part.place && p.transform.is_none() {
                return Err(Error::Record(format!(
                    "part '{}' has no transform",
                    p.part.name
                )));
            }
        }
        r.work_crs()?;
        Ok(r)
    }
}

/// 部分ごとに対応点から変換を解く。
pub fn solve(req: &Request) -> Result<Record, Error> {
    if !req.work_crs.is_work_crs() {
        return Err(Error::NotWorkCrs(req.work_crs.to_string()));
    }
    let parts = if req.parts.is_empty() {
        vec![Part::whole()]
    } else {
        req.parts.clone()
    };
    for g in &req.gcps {
        if !parts.iter().any(|p| p.name == g.part) {
            return Err(Error::UnknownPart(g.part.clone()));
        }
    }
    let to_work = Transformer::new(req.gcp_crs, req.work_crs)?;

    let mut out = Vec::with_capacity(parts.len());
    for part in parts {
        if !part.place {
            out.push(PartGeoref {
                part,
                model: None,
                transform: None,
                free_scale: None,
                control: vec![],
                check: vec![],
                control_error: None,
                check_error: None,
            });
            continue;
        }
        let mut control = vec![];
        let mut check = vec![];
        for g in req.gcps.iter().filter(|g| g.part == part.name) {
            let (x, y) = to_work.transform(g.map[0], g.map[1])?;
            let pr = PointResult {
                cad: g.cad,
                map: g.map,
                map_work: [x, y],
                residual: [0.0, 0.0],
            };
            match g.role {
                Role::Control => control.push(pr),
                Role::Check => check.push(pr),
            }
        }
        let pairs: Vec<Pair> = control
            .iter()
            .map(|p| Pair {
                cad: p.cad,
                map: p.map_work,
            })
            .collect();
        if pairs.len() < 2 {
            return Err(Error::PartTooFewPoints {
                part: part.name.clone(),
                got: pairs.len(),
            });
        }
        let model = match req.scale {
            ScaleMode::Fixed(s) => Model::Rigid { scale: s },
            ScaleMode::Free => Model::Similarity,
        };
        let t = fit::solve(&pairs, model)?;
        let free_scale = match req.scale {
            ScaleMode::Fixed(_) => Some(fit::solve(&pairs, Model::Similarity)?.scale),
            ScaleMode::Free => None,
        };
        let fill = |pts: &mut Vec<PointResult>| {
            for p in pts.iter_mut() {
                let (x, y) = t.apply(p.cad[0], p.cad[1]);
                p.residual = [p.map_work[0] - x, p.map_work[1] - y];
            }
            ErrorStats::of(&pts.iter().map(|p| p.residual).collect::<Vec<_>>())
        };
        let control_error = fill(&mut control);
        let check_error = fill(&mut check);
        out.push(PartGeoref {
            part,
            model: Some(model),
            transform: Some(t),
            free_scale,
            control,
            check,
            control_error,
            check_error,
        });
    }
    Ok(Record {
        cad2geojson_georef: RECORD_VERSION,
        work_crs: req.work_crs.to_string(),
        gcp_crs: req.gcp_crs.to_string(),
        parts: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 評価（docs/eval-dwg2geo.md §2.7.2）で求めたビッグシップ 2 階の変換を再現する。
    /// 対応点は円形レストランと大ホールの中心（図面は dwg2geo の図面座標 mm、地図は EPSG:6673）。
    #[test]
    fn reproduces_bigship_2f() {
        let req = Request {
            parts: vec![Part {
                name: "2F".into(),
                bbox: None,
                layers: vec![],
                place: true,
            }],
            gcps: vec![
                Gcp {
                    part: "2F".into(),
                    cad: [39255.06113716711, -65843.08762799996],
                    map: [-90816.4248194886, -63700.25321872406],
                    role: Role::Control,
                },
                Gcp {
                    part: "2F".into(),
                    cad: [117327.5343510328, -2802.344254158455],
                    map: [-90806.44423830819, -63798.36034277245],
                    role: Role::Control,
                },
            ],
            gcp_crs: Crs::from_epsg(6673).unwrap(),
            work_crs: Crs::from_epsg(6673).unwrap(),
            scale: ScaleMode::Fixed(0.001),
        };
        let r = solve(&req).unwrap();
        let t = r.parts[0].transform.unwrap();
        assert!(
            (t.rotation_deg - -123.11084085147061).abs() < 0.01,
            "{}",
            t.rotation_deg
        );
        assert!((t.translation[0] - -90739.91785442147).abs() < 0.01);
        assert!((t.translation[1] - -63702.477928490385).abs() < 0.01);
        assert!(r.parts[0].control_error.unwrap().rms < 1.0);
        // 2 点の相似変換では縮尺が 1.7% 小さく出る（屋根の倒れ込み）
        let fs = r.parts[0].free_scale.unwrap();
        assert!((fs / 0.001 - 0.983).abs() < 0.005, "{fs}");

        let back = Record::from_json(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn geographic_gcps_are_solved_in_work_crs() {
        // 図面の 1 単位 = 1 mm、回転 0、平面直角座標系 5 系の (-90000, -63000) を原点に置く図面
        let work = Crs::from_epsg(6673).unwrap();
        let back = Transformer::new(work, Crs::WGS84).unwrap();
        let cad = [
            [0.0, 0.0],
            [200_000.0, 0.0],
            [0.0, 150_000.0],
            [180_000.0, 120_000.0],
        ];
        let gcps: Vec<Gcp> = cad
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (lon, lat) = back
                    .transform(-90000.0 + c[0] / 1000.0, -63000.0 + c[1] / 1000.0)
                    .unwrap();
                Gcp {
                    part: "drawing".into(),
                    cad: *c,
                    map: [lon, lat],
                    role: if i == 3 { Role::Check } else { Role::Control },
                }
            })
            .collect();
        let req = Request {
            parts: vec![],
            gcps,
            gcp_crs: Crs::WGS84,
            work_crs: work,
            scale: ScaleMode::Fixed(0.001),
        };
        let r = solve(&req).unwrap();
        let p = &r.parts[0];
        let t = p.transform.unwrap();
        assert!(t.rotation_deg.abs() < 1e-7);
        assert!(
            (t.translation[0] + 90000.0).abs() < 1e-4 && (t.translation[1] + 63000.0).abs() < 1e-4
        );
        assert!(p.check_error.unwrap().max < 1e-4);
    }

    #[test]
    fn rejects_bad_requests() {
        let base = Request {
            parts: vec![],
            gcps: vec![],
            gcp_crs: Crs::WGS84,
            work_crs: Crs::WGS84,
            scale: ScaleMode::Free,
        };
        assert!(matches!(solve(&base), Err(Error::NotWorkCrs(_))));
        let req = Request {
            work_crs: Crs::from_epsg(6673).unwrap(),
            ..base.clone()
        };
        assert!(matches!(solve(&req), Err(Error::PartTooFewPoints { .. })));
        let req = Request {
            work_crs: Crs::from_epsg(6673).unwrap(),
            gcps: vec![Gcp {
                part: "1F".into(),
                cad: [0.0, 0.0],
                map: [0.0, 0.0],
                role: Role::Control,
            }],
            ..base
        };
        assert!(matches!(solve(&req), Err(Error::UnknownPart(_))));
    }
}
