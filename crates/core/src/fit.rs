//! 位置合わせの変換を対応点から解く。
//!
//! 変換は `map = s·R(θ)·cad + t`。剛体＋固定縮尺（s を単位から決め、θ と t を解く）と
//! 相似変換（s も解く）の 2 つだけを持つ。どちらも最小二乗の閉じた解がある。

use serde::{Deserialize, Serialize};

use crate::Error;

/// 図面座標 → 作業用の投影座標系（m）への相似変換。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Similarity {
    /// 図面の 1 単位が何 m か
    pub scale: f64,
    /// 反時計回りの回転（度）
    pub rotation_deg: f64,
    /// 平行移動（作業用の投影座標系、m）
    pub translation: [f64; 2],
}

impl Similarity {
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let (s, c) = self.rotation_deg.to_radians().sin_cos();
        (
            self.scale * (c * x - s * y) + self.translation[0],
            self.scale * (s * x + c * y) + self.translation[1],
        )
    }

    /// `self` を適用したあとに `next` を適用する変換。
    pub fn then(&self, next: &Similarity) -> Similarity {
        let (tx, ty) = next.apply(self.translation[0], self.translation[1]);
        Similarity {
            scale: self.scale * next.scale,
            rotation_deg: normalize_deg(self.rotation_deg + next.rotation_deg),
            translation: [tx, ty],
        }
    }
}

/// 変換のモデル。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Model {
    /// 縮尺を固定して回転と平行移動を解く（既定）
    Rigid { scale: f64 },
    /// 回転・平行移動・一様な縮尺を解く
    Similarity,
}

/// 対応点 1 組。`cad` は図面座標、`map` は作業用の投影座標系。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    pub cad: [f64; 2],
    pub map: [f64; 2],
}

/// 最小二乗で変換を解く。2 点以上が必要。
pub fn solve(pairs: &[Pair], model: Model) -> Result<Similarity, Error> {
    if pairs.len() < 2 {
        return Err(Error::TooFewPoints {
            need: 2,
            got: pairs.len(),
        });
    }
    let n = pairs.len() as f64;
    let mean = |f: fn(&Pair) -> [f64; 2]| {
        let (sx, sy) = pairs
            .iter()
            .map(f)
            .fold((0.0, 0.0), |(a, b), p| (a + p[0], b + p[1]));
        [sx / n, sy / n]
    };
    let mc = mean(|p| p.cad);
    let mm = mean(|p| p.map);

    // a = Σ(x·u + y·v), b = Σ(x·v − y·u), q = Σ(x² + y²)（重心からの座標）
    let (mut a, mut b, mut q) = (0.0, 0.0, 0.0);
    for p in pairs {
        let (x, y) = (p.cad[0] - mc[0], p.cad[1] - mc[1]);
        let (u, v) = (p.map[0] - mm[0], p.map[1] - mm[1]);
        a += x * u + y * v;
        b += x * v - y * u;
        q += x * x + y * y;
    }
    if q == 0.0 || (a == 0.0 && b == 0.0) {
        return Err(Error::DegeneratePoints);
    }
    let theta = b.atan2(a);
    let scale = match model {
        Model::Rigid { scale } => scale,
        Model::Similarity => a.hypot(b) / q,
    };
    let rot = Similarity {
        scale,
        rotation_deg: theta.to_degrees(),
        translation: [0.0, 0.0],
    };
    let (rx, ry) = rot.apply(mc[0], mc[1]);
    Ok(Similarity {
        translation: [mm[0] - rx, mm[1] - ry],
        ..rot
    })
}

/// 点ごとの誤差（m）。`map - 変換(cad)`。
pub fn residuals(t: &Similarity, pairs: &[Pair]) -> Vec<[f64; 2]> {
    pairs
        .iter()
        .map(|p| {
            let (x, y) = t.apply(p.cad[0], p.cad[1]);
            [p.map[0] - x, p.map[1] - y]
        })
        .collect()
}

/// 誤差の要約（m）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ErrorStats {
    pub rms: f64,
    pub max: f64,
}

impl ErrorStats {
    pub fn of(res: &[[f64; 2]]) -> Option<ErrorStats> {
        if res.is_empty() {
            return None;
        }
        let d: Vec<f64> = res.iter().map(|r| r[0].hypot(r[1])).collect();
        let rms = (d.iter().map(|x| x * x).sum::<f64>() / d.len() as f64).sqrt();
        Some(ErrorStats {
            rms,
            max: d.iter().cloned().fold(0.0, f64::max),
        })
    }
}

pub fn normalize_deg(d: f64) -> f64 {
    let r = d.rem_euclid(360.0);
    if r > 180.0 { r - 360.0 } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn rigid_recovers_known_transform() {
        let t = Similarity {
            scale: 0.001,
            rotation_deg: 30.0,
            translation: [100.0, -200.0],
        };
        let cad = [[0.0, 0.0], [10000.0, 0.0], [0.0, 5000.0], [7000.0, 3000.0]];
        let pairs: Vec<Pair> = cad
            .iter()
            .map(|c| {
                let (x, y) = t.apply(c[0], c[1]);
                Pair {
                    cad: *c,
                    map: [x, y],
                }
            })
            .collect();
        for model in [Model::Rigid { scale: 0.001 }, Model::Similarity] {
            let f = solve(&pairs, model).unwrap();
            assert!(close(f.scale, 0.001, 1e-12));
            assert!(close(f.rotation_deg, 30.0, 1e-9));
            assert!(close(f.translation[0], 100.0, 1e-9) && close(f.translation[1], -200.0, 1e-9));
            assert!(ErrorStats::of(&residuals(&f, &pairs)).unwrap().max < 1e-9);
        }
    }

    #[test]
    fn rigid_two_points_hand_computed() {
        // 図面の (0,0)→(1000,0) が、地図の (10,10)→(10,11.2) に写る。
        // 固定縮尺 0.001 なら回転 90°、図面の長さ 1 m が地図の 1.2 m に収まらないので
        // 中点を合わせ、両端に ±0.1 m の誤差が残る。
        let pairs = [
            Pair {
                cad: [0.0, 0.0],
                map: [10.0, 10.0],
            },
            Pair {
                cad: [1000.0, 0.0],
                map: [10.0, 11.2],
            },
        ];
        let f = solve(&pairs, Model::Rigid { scale: 0.001 }).unwrap();
        assert!(close(f.rotation_deg, 90.0, 1e-9));
        assert!(close(f.translation[0], 10.0, 1e-9) && close(f.translation[1], 10.1, 1e-9));
        let r = residuals(&f, &pairs);
        assert!(close(r[0][1], -0.1, 1e-9) && close(r[1][1], 0.1, 1e-9));
        let s = solve(&pairs, Model::Similarity).unwrap();
        assert!(close(s.scale, 0.0012, 1e-12));
    }

    #[test]
    fn errors() {
        let p = Pair {
            cad: [0.0, 0.0],
            map: [0.0, 0.0],
        };
        assert!(matches!(
            solve(&[p], Model::Similarity),
            Err(Error::TooFewPoints { .. })
        ));
        assert!(matches!(
            solve(&[p, p], Model::Similarity),
            Err(Error::DegeneratePoints)
        ));
    }

    #[test]
    fn compose() {
        let a = Similarity {
            scale: 2.0,
            rotation_deg: 90.0,
            translation: [1.0, 0.0],
        };
        let b = Similarity {
            scale: 0.5,
            rotation_deg: -45.0,
            translation: [0.0, 3.0],
        };
        let ab = a.then(&b);
        let (x, y) = a.apply(3.0, 4.0);
        let (x1, y1) = b.apply(x, y);
        let (x2, y2) = ab.apply(3.0, 4.0);
        assert!(close(x1, x2, 1e-12) && close(y1, y2, 1e-12));
    }
}
