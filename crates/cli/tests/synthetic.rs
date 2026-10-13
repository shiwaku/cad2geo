//! 答えが分かっている合成の図面（testdata/synthetic、tools/gen_testdata で生成）で、
//! 部分分け・`follow`・位置合わせを CLI ごと確かめる。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use cad2geojson_core::georef::Record;
use cad2geojson_gen_testdata as gen_data;
use gen_data::Floor;
use serde_json::Value;

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/synthetic")
        .join(name)
}

fn out_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("cad2geojson-synthetic-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `georef` を回し、記録と出力の図形を返す。
fn georef(dxf: &str, out: &Path) -> (Record, Vec<Value>) {
    let stem = dxf.trim_end_matches(".dxf");
    let output = out.join(format!("{stem}.geojson"));
    let record = out.join(format!("{stem}.georef.json"));
    let r = Command::new(env!("CARGO_BIN_EXE_cad2geojson"))
        .arg("georef")
        .arg(data(dxf))
        .arg("--gcp")
        .arg(data("gcp.csv"))
        .arg("--parts")
        .arg(data("parts.json"))
        .args(["--work-crs", gen_data::WORK_CRS, "--units", "mm", "-o"])
        .arg(&output)
        .arg("--record")
        .arg(&record)
        .output()
        .unwrap();
    assert!(
        r.status.success(),
        "georef {dxf} failed:\n{}",
        String::from_utf8_lossy(&r.stderr)
    );
    let rec = Record::from_json(&std::fs::read_to_string(&record).unwrap()).unwrap();
    let gj: Value = serde_json::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    (rec, gj["features"].as_array().unwrap().clone())
}

fn check_record(rec: &Record) {
    let part = |n: &str| rec.parts.iter().find(|p| p.part.name == n).unwrap();
    assert!(part("frame").transform.is_none());
    for floor in [Floor::F2, Floor::F1] {
        let p = part(floor.name());
        let t = p.transform.unwrap();
        let want = floor.truth();
        assert!((t.rotation_deg - want.rotation_deg).abs() < 1e-6, "{t:?}");
        assert!((t.scale - gen_data::UNITS_M).abs() < 1e-12);
        for k in 0..2 {
            assert!(
                (t.translation[k] - want.translation[k]).abs() < 1e-4,
                "{t:?}"
            );
        }
        assert!(p.check_error.unwrap().max < 1e-3);
    }
    let rel = part("1F").relation.as_ref().unwrap();
    assert_eq!(rel.to, "2F");
    assert_eq!(
        rel.pairs.len(),
        gen_data::GRID_X.len() * gen_data::GRID_Y.len()
    );
    assert!(rel.runner_up * 2 < rel.pairs.len());
    assert!(rel.residual_max < 1.0);
}

fn texts(features: &[Value]) -> BTreeSet<(String, String)> {
    features
        .iter()
        .filter_map(|f| {
            let p = &f["properties"];
            Some((p["part"].as_str()?.into(), p["text"].as_str()?.into()))
        })
        .collect()
}

#[test]
fn r12_and_r2010_place_both_floors() {
    let out = out_dir();
    let (r12, f12) = georef("plan_r12.dxf", &out);
    let (r2010, f2010) = georef("plan_r2010.dxf", &out);
    check_record(&r12);
    check_record(&r2010);

    // 図枠は載せない。文字（cp932 / UTF-8）はどちらも同じに読める
    let t = texts(&f12);
    assert!(t.contains(&("1F".into(), "エントランスホール".into())));
    assert!(t.contains(&("2F".into(), "会議室Ａ".into())));
    assert!(t.iter().all(|(_, s)| !s.contains("合成テスト建物")));
    assert_eq!(t, texts(&f2010));
    assert_eq!(f12.len(), f2010.len());
    assert!(f12.iter().all(|f| f["properties"]["layer"] != "図枠"));

    std::fs::remove_dir_all(&out).unwrap();
}
