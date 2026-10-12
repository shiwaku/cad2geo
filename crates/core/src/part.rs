//! 図面を部分（1 階・2 階・図枠など）に分ける。

use serde::{Deserialize, Serialize};

/// 図面の部分。図形は外接矩形の中心で振り分ける。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    /// 図面座標の範囲 [xmin, ymin, xmax, ymax]。無ければ範囲で絞らない
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    /// レイヤの集合。空なら全レイヤ
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<String>,
    /// 地図に載せるか（図枠・凡例などは false）
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub place: bool,
    /// 位置を対応点でなく、共通する円（柱など）でこの部分に合わせて決める（1 階を 2 階に合わせるなど）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow: Option<String>,
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

impl Part {
    /// 部分を定義しないときの、図面全体を 1 つとみなす部分。
    pub fn whole() -> Part {
        Part {
            name: "drawing".into(),
            bbox: None,
            layers: vec![],
            place: true,
            follow: None,
        }
    }

    pub fn contains(&self, center: [f64; 2], layer: Option<&str>) -> bool {
        let in_box = self.bbox.is_none_or(|b| {
            b[0] <= center[0] && center[0] <= b[2] && b[1] <= center[1] && center[1] <= b[3]
        });
        let in_layer =
            self.layers.is_empty() || layer.is_some_and(|l| self.layers.iter().any(|x| x == l));
        in_box && in_layer
    }
}

/// 先に書かれた部分を優先して振り分ける。どれにも入らなければ `None`。
pub fn assign(parts: &[Part], center: [f64; 2], layer: Option<&str>) -> Option<usize> {
    parts.iter().position(|p| p.contains(center, layer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_match_wins() {
        let parts = vec![
            Part {
                name: "frame".into(),
                bbox: None,
                layers: vec!["WAKU".into()],
                place: false,
                follow: None,
            },
            Part {
                name: "2F".into(),
                bbox: Some([0.0, -100.0, 100.0, 0.0]),
                layers: vec![],
                place: true,
                follow: None,
            },
            Part {
                name: "1F".into(),
                bbox: Some([0.0, 0.0, 100.0, 100.0]),
                layers: vec![],
                place: true,
                follow: None,
            },
        ];
        assert_eq!(assign(&parts, [50.0, -50.0], Some("WALL")), Some(1));
        assert_eq!(assign(&parts, [50.0, 50.0], None), Some(2));
        assert_eq!(assign(&parts, [50.0, 50.0], Some("WAKU")), Some(0));
        assert_eq!(assign(&parts, [500.0, 50.0], Some("WALL")), None);
    }

    #[test]
    fn json_defaults() {
        let p: Part = serde_json::from_str(r#"{"name":"1F","bbox":[0,0,1,1]}"#).unwrap();
        assert!(p.place && p.layers.is_empty());
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            r#"{"name":"1F","bbox":[0.0,0.0,1.0,1.0]}"#
        );
    }
}
