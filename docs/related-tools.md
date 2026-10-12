# 関連ツールの調査メモ

cad2geo を作る過程で調べたツールと、cad2geo との関係。

| ツール | 種類 | DWG | DXF | cad2geo との関係 |
|---|---|---|---|---|
| [dwg2geo](https://github.com/milkway/dwg2geo)（MIT） | DWG → GeoJSON の変換器 | 読む | 読まない | **土台**。DWG の図形の取り出しを任せる（[評価](eval-dwg2geo.md)） |
| [acadrust](https://github.com/hakanaktt/acadrust)（MPL-2.0） | DWG / DXF の読み書きライブラリ（Rust） | 読み書き | 読み書き | **DXF の読み込みに使う**（DXF → メモリ上の DWG → dwg2geo） |
| GDAL（OGR DXF ドライバ） | 地理空間データの変換 | 読まない | 読む | 比較に使用。R12（文字コードの記録なし）で GeoJSON が壊れ、R2010 で文字化けした（[評価](eval-dwg2geo.md)） |
| ODA File Converter（無料、ODA） | DWG / DXF の版の変換 | 読み書き | 読み書き | 検証用の DWG を作るのに使用。読めない DWG を DXF に変換する経路として案内する |
| LibreDWG | DWG の読み書きライブラリ（C） | 読み書き | 読み書き | dwg2geo の外部バックエンドが使う。cad2geo では使わない |
| [SolveCraft](https://github.com/bherbruck/solvecraft)（MIT / Apache-2.0） | 3D パラメトリック CAD（Rust、Fusion 風） | **非対応** | 板金の展開図の書き出しのみ | **関係なし**（下記） |

## SolveCraft（2026-10-12 確認）

Rust 製のオープンソースの 3D パラメトリック CAD（[fabscene の記事](https://fabscene.com/new/make/solvecraft-open-source-parametric-3d-cad-rust-fusion-76-parts-mcp/)）。

- 読み込み: STEP（AP203 / AP214 / AP242）、IGES、3MF・STL・OBJ（メッシュとして）、独自形式（.solvecraft、JSON）
- 書き出し: STEP（AP242）、IGES 5.3、3MF、STL、OBJ、独自形式
- DXF: 板金の展開図（線・円弧・円の外形と曲げ表）の書き出しのみ。DXF の読み込みは無い
- DWG: README・記事とも言及なし

STEP を中心とした 3D 形状の CAD で、2D の建築・土木図面（DWG / DXF）を扱う道具ではない。cad2geo の入力の作り手にも、検証用の図面の出どころにもならない。
