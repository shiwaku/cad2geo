# cad2geojson

CAD 図面（DWG / DXF）を、Web 地図（MapLibre GL JS）に**正しい位置で**重ねられる GeoJSON にするためのツール。

> **状態: 開発中（Phase 1）。** DWG / DXF を読み、部分に分けて位置合わせし、WGS84 の GeoJSON にする CLI ができた。設計は [docs/design.md](docs/design.md)、dwg2geo の評価は [docs/eval-dwg2geo.md](docs/eval-dwg2geo.md)、調べた関連ツールは [docs/related-tools.md](docs/related-tools.md) にある。

## 方針

DWG を読んで図形を取り出す部分は、既存の OSS [dwg2geo](https://github.com/milkway/dwg2geo)（MIT、内部で [acadrust](https://github.com/hakanaktt/acadrust) を使う）に任せる。cad2geojson は、dwg2geo に無い **「地図に正しく載せる」部分**を作る。

| cad2geojson が作るもの | 内容 |
|---|---|
| 図面を部分に分ける | 1 枚に並んだ 1 階・2 階・図枠などを分け、部分ごとに位置合わせする。階どうしは共通する柱などで相対位置を決める |
| 位置合わせ | 縮尺を単位から固定した剛体変換（既定）と相似変換。対応点はどの CRS で受け取っても作業用の投影座標系（平面直角座標系）で解く。検証点の誤差も出す |
| 座標系変換 | 純 Rust の proj4rs で WGS84 へ。ブラウザでも動く |
| 文字の属性の補正 | 文字の回転角・文字高を位置合わせに合わせて直す |
| 対応点を探す | 地図と図面を並べて点を打つ画面、図形の中心の自動抽出 |
| DXF | acadrust で読み、dwg2geo と同じ取り出し方で変換する。日本の DXF（Jw_cad 由来の R12 など）の文字コードを判定する |

上流（dwg2geo・acadrust）の不足は、上流の公開 API だけを使って cad2geojson の中で補う。

```
DWG ───────────────────┬─ dwg2geo ─→ 図面座標の GeoJSON ─→ 部分に分ける ─→ 位置合わせ ─→ WGS84 へ ─→ GeoJSON
DXF ─ acadrust → DWG ──┘                                                       ↑
                                                                          対応点を探す
```

- **CLI**（Rust）: dwg2geo と acadrust を crate として使う
- **ブラウザ版**（Vite + MapLibre）: dwg2geo の npm（WASM）で変換し、cad2geojson の位置合わせを WASM で動かす。図面をブラウザの外に送らない

## 使い方

```sh
# 1. 図面座標の GeoJSON にして、対応点の図面側の座標を QGIS などで拾う
cad2geojson convert plan.dxf -o plan.local.geojson

# 2. 対応点で位置合わせし、WGS84 の GeoJSON にする（入力は DWG / DXF / 図面座標の GeoJSON）
cad2geojson georef plan.dxf \
  --gcp gcp.csv --work-crs EPSG:6673 --units mm \
  --parts parts.json -o plan.geojson
```

`gcp.csv`（`role` は `control`（既定）か `check`。検証点は変換を解くのに使わず、誤差だけを出す）:

```csv
part,cad_x,cad_y,map_x,map_y,role
2F,39255.06,-65843.09,-90816.425,-63700.253,control
2F,117327.53,-2802.34,-90806.444,-63798.360,control
```

`parts.json`（図形は外接矩形の中心で、先に書いた部分から振り分ける。`place: false` の部分は出力しない）:

```json
{"parts": [
  {"name": "frame", "bbox": [2363, 162928, 66520, 193193], "place": false},
  {"name": "1F", "bbox": [-1e9, 60000, 1e9, 1e9]},
  {"name": "2F", "bbox": [-1e9, -1e9, 1e9, 60000], "layers": []}
]}
```

| オプション | 内容 |
|---|---|
| `--work-crs` | 変換を解く平面直角座標系（必須。系は自動で選ばない） |
| `--gcp-crs` | 対応点の地図側の CRS（既定は `--work-crs`）。経緯度（EPSG:4326）で渡しても作業用の座標系で解く |
| `--units` | 図面の単位（`mm` など）。縮尺をこれに固定し、回転と平行移動だけを解く |
| `--free-scale` | 縮尺も解く（相似変換）。単位が分からないとき |
| `--record` | 位置合わせの記録の出力先（既定は `<出力>.georef.json`） |
| `--dxf-encoding` | DXF の文字コード（`ANSI_932` など DXF の表記）。既定では、記録が無い R12 などは中身が cp932 として妥当なら `ANSI_932` で読む |

- 部分ごとの回転・縮尺・平行移動、対応点と検証点の誤差、縮尺を固定しなかった場合の縮尺を表示し、記録（JSON）に書き出す
- `cad2geojson apply plan.dxf --record plan.geojson.georef.json -o plan.geojson` で、記録から同じ変換を再現する
- 出力は WGS84 の GeoJSON（`.geojsonl` / `.ndjson` なら 1 行 1 図形）。各図形に `part` 属性を付ける
- 文字は `text_rotation_deg` を真北基準（子午線収差も補正）・反時計回りの度に、`text_height` を m に直す。元の値は `cad_text_rotation_deg` / `cad_text_height` に残す。MapLibre の `text-rotate` は時計回りなので、符号を反転して使う

## ビルド

```sh
cargo build --release   # target/release/cad2geojson
```

Windows の Git Bash では `/usr/bin/link` が MSVC の `link.exe` より先に見つかるので、cargo は PowerShell から実行する。
## DWG の仕様は公開されていない

DWG は Autodesk の独自バイナリ形式で、仕様は公開されていない。公式に読み書きできるのは Autodesk 製品と、同社の有償 SDK（RealDWG）のライセンス製品である。OSS が DWG を読めるのは、Open Design Alliance（ODA）が解析結果を「Open Design Specification for .dwg files」として無料公開しているなど、リバースエンジニアリングの蓄積があるためだ。一方 DXF は Autodesk が仕様（DXF Reference）を公開している。

- DWG は**完全に読める保証がない**。新しい形式（AC1032）や珍しいエンティティは欠ける可能性がある
- **実際の CAD ソフトが書き出した DWG** で検証する
- 直接読めない DWG は、ODA File Converter（無料）で DXF に変換してから読む経路を案内する
- 「DWG」の名称は Autodesk との商標問題の経緯があるため、「DWG ファイルを読み込める」という説明的な表現にとどめ、公式対応を連想させる書き方はしない

## 検証データ

実在する図面で検証する。第三者の図面は同梱せず、取得元・取得手順・利用条件を [testdata/README.md](testdata/README.md) に書く。

- **建築図面**: 米子コンベンションセンター（ビッグシップ）の施設図面（DXF）、Autodesk の AutoCAD サンプル（DWG）
- **位置合わせの基準**: G空間情報センター 鳥取県「空中写真（オルソ画像）」（公共データ利用規約 第 1.0 版）
- **測量図面**: BODIK の自治体基本図 DXF（CC BY）、神奈川県 3 次元点群の等高線（DXF、CC BY 4.0）

## 背景

2024 年 4 月に、顧客提供の DWG（平面直角座標系 9 系、mm 単位、AC1032）を次の手順で GIS データ化した。

1. AutoCAD で DWG → DXF（AutoCAD 2010 形式。新しい形式では日本語が文字化けした）
2. `ogr2ogr` で DXF → GeoJSON（`DXF_FEATURE_LIMIT_PER_BLOCK=-1`）
3. geopandas で座標を 0.001 倍し、EPSG:6677 を付与

市販 CAD を使わず、DWG / DXF から地図に正しく載るデータまでを 1 本で通すのがこのツールの狙い。

## ライセンス

MIT。依存する [dwg2geo](https://github.com/milkway/dwg2geo) は MIT、[acadrust](https://github.com/hakanaktt/acadrust) は MPL-2.0。
