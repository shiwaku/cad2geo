//! 座標系。EPSG データベースは持たず、日本で使うものだけを表で持つ。

use std::fmt;
use std::str::FromStr;

use proj4rs::proj::Proj;

use crate::Error;

/// 平面直角座標系の原点（緯度, 経度）。系 1〜19。
const PLANE_ORIGINS: [(f64, f64); 19] = [
    (33.0, 129.5),
    (33.0, 131.0),
    (36.0, 132.0 + 10.0 / 60.0),
    (33.0, 133.5),
    (36.0, 134.0 + 20.0 / 60.0),
    (36.0, 136.0),
    (36.0, 137.0 + 10.0 / 60.0),
    (36.0, 138.5),
    (36.0, 139.0 + 50.0 / 60.0),
    (40.0, 140.0 + 50.0 / 60.0),
    (44.0, 140.25),
    (44.0, 142.25),
    (44.0, 144.25),
    (26.0, 142.0),
    (26.0, 127.5),
    (26.0, 124.0),
    (26.0, 131.0),
    (20.0, 136.0),
    (26.0, 154.0),
];

/// 扱える座標系。JGD2000 / JGD2011 と WGS84 は同一とみなす（Web 地図の表示では問題にならない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crs {
    /// 経緯度（EPSG:4326 / 6668 / 4612）
    Geographic { epsg: u32 },
    /// Web メルカトル（EPSG:3857）
    WebMercator,
    /// 平面直角座標系（JGD2011: EPSG:6669〜6687、JGD2000: EPSG:2443〜2461）
    JapanPlane { epsg: u32, zone: u8 },
}

impl Crs {
    pub const WGS84: Crs = Crs::Geographic { epsg: 4326 };

    pub fn from_epsg(epsg: u32) -> Result<Crs, Error> {
        match epsg {
            4326 | 6668 | 4612 => Ok(Crs::Geographic { epsg }),
            3857 => Ok(Crs::WebMercator),
            6669..=6687 => Ok(Crs::JapanPlane {
                epsg,
                zone: (epsg - 6668) as u8,
            }),
            2443..=2461 => Ok(Crs::JapanPlane {
                epsg,
                zone: (epsg - 2442) as u8,
            }),
            _ => Err(Error::UnsupportedCrs(format!("EPSG:{epsg}"))),
        }
    }

    pub fn epsg(&self) -> u32 {
        match *self {
            Crs::Geographic { epsg } | Crs::JapanPlane { epsg, .. } => epsg,
            Crs::WebMercator => 3857,
        }
    }

    pub fn is_geographic(&self) -> bool {
        matches!(self, Crs::Geographic { .. })
    }

    /// 位置合わせを解ける投影座標系か（Web メルカトルは縮尺が緯度で変わるので不可）。
    pub fn is_work_crs(&self) -> bool {
        matches!(self, Crs::JapanPlane { .. })
    }

    fn proj_string(&self) -> String {
        match *self {
            Crs::Geographic { .. } => "+proj=longlat +ellps=GRS80 +towgs84=0,0,0 +no_defs".into(),
            Crs::WebMercator => concat!(
                "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 +k=1",
                " +units=m +nadgrids=@null +no_defs"
            )
            .into(),
            Crs::JapanPlane { zone, .. } => {
                let (lat0, lon0) = PLANE_ORIGINS[zone as usize - 1];
                format!(
                    "+proj=tmerc +lat_0={lat0} +lon_0={lon0} +k=0.9999 +x_0=0 +y_0=0 \
                     +ellps=GRS80 +towgs84=0,0,0 +units=m +no_defs"
                )
            }
        }
    }

    fn proj(&self) -> Result<Proj, Error> {
        Proj::from_proj_string(&self.proj_string()).map_err(|e| Error::Projection(e.to_string()))
    }
}

impl FromStr for Crs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Crs, Error> {
        let t = s.trim();
        let code = t
            .strip_prefix("EPSG:")
            .or_else(|| t.strip_prefix("epsg:"))
            .unwrap_or(t);
        let epsg = code
            .parse::<u32>()
            .map_err(|_| Error::UnsupportedCrs(s.into()))?;
        Crs::from_epsg(epsg)
    }
}

impl fmt::Display for Crs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EPSG:{}", self.epsg())
    }
}

/// 2 つの座標系の間の変換。経緯度は度で受け渡す（x = 経度, y = 緯度）。
pub struct Transformer {
    src: Proj,
    dst: Proj,
    src_geo: bool,
    dst_geo: bool,
    identity: bool,
}

impl Transformer {
    pub fn new(src: Crs, dst: Crs) -> Result<Transformer, Error> {
        let identity = src == dst || (src.is_geographic() && dst.is_geographic());
        Ok(Transformer {
            src: src.proj()?,
            dst: dst.proj()?,
            src_geo: src.is_geographic(),
            dst_geo: dst.is_geographic(),
            identity,
        })
    }

    pub fn transform(&self, x: f64, y: f64) -> Result<(f64, f64), Error> {
        if self.identity {
            return Ok((x, y));
        }
        let mut p = if self.src_geo {
            (x.to_radians(), y.to_radians(), 0.0)
        } else {
            (x, y, 0.0)
        };
        proj4rs::transform::transform(&self.src, &self.dst, &mut p)
            .map_err(|e| Error::Projection(e.to_string()))?;
        if self.dst_geo {
            Ok((p.0.to_degrees(), p.1.to_degrees()))
        } else {
            Ok((p.0, p.1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 期待値は PROJ（pyproj 3.7.0）で求めた。
    #[test]
    fn plane_to_wgs84_matches_proj() {
        let cases = [
            (6673, -90739.918, -63702.478, 133.3341031776, 35.4216727595),
            (6673, 0.0, 0.0, 134.3333333333, 36.0),
            (6677, -12345.678, -34567.891, 139.6969291834, 35.6883459560),
            (6680, 100000.0, -50000.0, 143.4874525218, 43.5432447238),
            (2447, -90739.918, -63702.478, 133.3341031776, 35.4216727595),
        ];
        for (epsg, x, y, lon, lat) in cases {
            let t = Transformer::new(Crs::from_epsg(epsg).unwrap(), Crs::WGS84).unwrap();
            let (lo, la) = t.transform(x, y).unwrap();
            // 1e-8 度 ≒ 1 mm
            assert!(
                (lo - lon).abs() < 1e-8 && (la - lat).abs() < 1e-8,
                "EPSG:{epsg} {lo} {la}"
            );
        }
    }

    #[test]
    fn roundtrip_wgs84_plane() {
        let to = Transformer::new(Crs::WGS84, Crs::from_epsg(6673).unwrap()).unwrap();
        let back = Transformer::new(Crs::from_epsg(6673).unwrap(), Crs::WGS84).unwrap();
        let (x, y) = to.transform(133.3341031776, 35.4216727595).unwrap();
        assert!(
            (x + 90739.918).abs() < 1e-3 && (y + 63702.478).abs() < 1e-3,
            "{x} {y}"
        );
        let (lo, la) = back.transform(x, y).unwrap();
        assert!((lo - 133.3341031776).abs() < 1e-10 && (la - 35.4216727595).abs() < 1e-10);
    }

    #[test]
    fn parse() {
        assert_eq!(
            "EPSG:6673".parse::<Crs>().unwrap(),
            Crs::JapanPlane {
                epsg: 6673,
                zone: 5
            }
        );
        assert_eq!(
            "2451".parse::<Crs>().unwrap(),
            Crs::JapanPlane {
                epsg: 2451,
                zone: 9
            }
        );
        assert!("EPSG:32654".parse::<Crs>().is_err());
        assert!(!Crs::WebMercator.is_work_crs());
    }
}
