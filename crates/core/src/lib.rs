//! cad2geo の中核。図面座標の GeoJSON を部分に分け、部分ごとに位置合わせし、WGS84 にする。
//! ファイル I/O は持たない（CLI と WASM で共有する）。

pub mod crs;
pub mod fit;
pub mod geojson;
pub mod georef;
pub mod part;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported CRS: {0} (supported: EPSG:4326, 6668, 4612, 3857, 6669-6687, 2443-2461)")]
    UnsupportedCrs(String),
    #[error(
        "{0} cannot be used as the work CRS; use a Japan Plane Rectangular CRS (EPSG:6669-6687)"
    )]
    NotWorkCrs(String),
    #[error("projection error: {0}")]
    Projection(String),
    #[error("at least {need} control points are required, got {got}")]
    TooFewPoints { need: usize, got: usize },
    #[error("part '{part}' needs at least 2 control points, got {got}")]
    PartTooFewPoints { part: String, got: usize },
    #[error("control points are degenerate (coincident)")]
    DegeneratePoints,
    #[error("control point refers to unknown part '{0}'")]
    UnknownPart(String),
    #[error("invalid georef record: {0}")]
    Record(String),
    #[error("invalid GeoJSON: {0}")]
    GeoJson(String),
}
