# cad2geo

CAD 図面（DWG / DXF）を、Web 地図（MapLibre GL JS）に**正しい位置で**重ねられる GeoJSON にするためのツール。

> **状態: 設計段階。** 実装はまだ無い。設計は [docs/design.md](docs/design.md)、土台にする dwg2geo の評価は [docs/eval-dwg2geo.md](docs/eval-dwg2geo.md) にある。

## 方針

DWG を読んで図形を取り出す部分は、既存の OSS [dwg2geo](https://github.com/milkway/dwg2geo)（MIT、内部で [acadrust](https://github.com/hakanaktt/acadrust) を使う）に任せる。cad2geo は、dwg2geo に無い **「地図に正しく載せる」部分**を作る。

| cad2geo が作るもの | 内容 |
|---|---|
| 図面を部分に分ける | 1 枚に並んだ 1 階・2 階・図枠などを分け、部分ごとに位置合わせする。階どうしは共通する柱などで相対位置を決める |
| 位置合わせ | 縮尺を単位から固定した剛体変換（既定）と相似変換。対応点はどの CRS で受け取っても作業用の投影座標系（平面直角座標系）で解く。検証点の誤差も出す |
| 座標系変換 | 純 Rust の proj4rs で WGS84 へ。ブラウザでも動く |
| 文字の属性の補正 | 文字の回転角・文字高を位置合わせに合わせて直す |
| 対応点を探す | 地図と図面を並べて点を打つ画面、図形の中心の自動抽出 |
| DXF | 上流（dwg2geo）への PR を基本に、日本の DXF（Jw_cad 由来の R12 など）の文字コードを正しく扱う |

変換器として当然あるべきもの（DXF の入力、文字の揃え位置など）は、上流に PR を送る。

```
DWG / DXF ─ dwg2geo ─→ 図面座標の GeoJSON ─→ 部分に分ける ─→ 位置合わせ ─→ WGS84 へ ─→ GeoJSON
                                                              ↑
                                                         対応点を探す
```

- **CLI**（Rust）: dwg2geo を crate として使う
- **ブラウザ版**（Vite + MapLibre）: dwg2geo の npm（WASM）で変換し、cad2geo の位置合わせを WASM で動かす。図面をブラウザの外に送らない

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

MIT
