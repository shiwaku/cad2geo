# cad2geo

CAD 図面（DWG / DXF）を地理空間データ（GeoJSON / FlatGeobuf）に変換し、Web 地図（MapLibre GL JS）に重ねるためのツール。

> **状態: 設計段階。** 実装はまだ無い。設計は [docs/design.md](docs/design.md) にある。

## 目的

- DWG と DXF の両方を、同じコードで読む
- 単位（mm など）と位置合わせを扱い、地図と正しく重なるデータを出す
  - 測量図面: 平面直角座標系などの座標が既に付いている。単位を直し、EPSG を指定するだけ
  - 建築平面図: 原点 (0,0) 基準のローカル座標。基準点・回転角・縮尺、または対応点で地図上に配置する
- 変換サーバーを使わずにブラウザだけでも動かす

## 構成（予定）

```
cad2geo
├─ core  (Rust, acadrust)
│   DWG/DXF 読込 → エンティティを GeoJSON ジオメトリに変換
│     LINE / LWPOLYLINE / POLYLINE → LineString (閉じていれば Polygon も選択可)
│     ARC / CIRCLE / ELLIPSE / SPLINE → 折れ線化
│     TEXT / MTEXT → Point + 文字列属性
│     INSERT → ブロック展開 (入れ子・属性付きブロック含む)
│   属性: レイヤ名・色・線種・ハンドル・ブロック名
│   単位の解決 ($INSUNITS、無ければ引数で指定) → 位置合わせ → 出力 CRS
├─ cli   ネイティブ実行。GeoJSON / FlatGeobuf 出力 (PMTiles 化は tippecanoe)
└─ web   WASM + MapLibre。ファイルをドロップ → 地図に重ねる → ドラッグで位置合わせ → GeoJSON 保存
```

## DWG/DXF の読み込みライブラリ

[acadrust](https://github.com/hakanaktt/acadrust) を使う。

| 項目 | 内容 |
|---|---|
| ライセンス | MPL-2.0 |
| 形式 | DWG・DXF（ASCII / バイナリ）の読み書き |
| バージョン | AC1009 (R12) 〜 AC1032 (2018+) |
| WASM | `wasm32-unknown-unknown` でビルドできることを CI で確認している |
| 文字コード | 2007 より前の図面のコードページ（約 40 種）を自動判定 |

### DWG の仕様は公開されていない

DWG は Autodesk の独自バイナリ形式で、仕様は公開されていない。公式に読み書きできるのは Autodesk 製品と、同社の有償 SDK（RealDWG）のライセンス製品である。OSS が DWG を読めるのは、Open Design Alliance（ODA）が解析結果を「Open Design Specification for .dwg files」として無料公開しているなど、リバースエンジニアリングの蓄積があるためだ。一方 DXF は Autodesk が仕様（DXF Reference）を公開している。

そのため、このツールでは次のように扱う。

- DWG は**完全に読める保証がない**。新しい形式（AC1032）や珍しいエンティティは欠ける可能性がある
- **実際の CAD ソフトが書き出した DWG** で検証する
- 直接読めない DWG は、ODA File Converter（無料）で DXF に変換してから読む経路を残す
- 「DWG」の名称は Autodesk との商標問題の経緯があるため、「DWG ファイルを読み込める」という説明的な表現にとどめ、公式対応を連想させる書き方はしない

### 比較した他の選択肢

- **LibreDWG / libredwg-web**: GPL。DWG は 2018 形式まで
- **GDAL (libopencad)**: DWG は R2000 まで。GDAL の DWG ドライバ本体は ODA Drawings SDK（有償）が必要
- **ODA File Converter**: 無料。DWG/DXF のバージョン変換のみ。前処理用の外部コマンドとしては使える
- **ODA Drawings SDK**: 有償（ODA 会員）

## 検証データ

実在する図面で検証する。第三者の図面は同梱せず、取得元・取得手順・利用条件を [testdata/README.md](testdata/README.md) に書く。

- **建築図面**: 米子コンベンションセンター（ビッグシップ）の施設図面（DXF）、Autodesk の AutoCAD サンプル（DWG）
- **測量図面**: 大阪市地形図（DXF）、神奈川県 3 次元点群の等高線（DXF、CC BY 4.0）

## 背景

2024 年 4 月に、顧客提供の DWG（平面直角座標系 9 系、mm 単位、AC1032）を次の手順で GIS データ化した。

1. AutoCAD で DWG → DXF（AutoCAD 2010 形式。新しい形式では日本語が文字化けした）
2. `ogr2ogr` で DXF → GeoJSON（`DXF_FEATURE_LIMIT_PER_BLOCK=-1`）
3. geopandas で座標を 0.001 倍し、EPSG:6677 を付与

市販 CAD を使わず、DWG/DXF から地図に載るデータまでを 1 本で通すのがこのツールの狙い。

## ライセンス

MIT
