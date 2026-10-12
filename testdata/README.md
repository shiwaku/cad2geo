# 検証データ

このディレクトリは `README.md` 以外を追跡しない。第三者の図面は取得手順だけを書き、ファイルは各自で取得する。

## 建築図面

### 合成した建築平面図（`synthetic/`）

`tools/gen_testdata` で生成する（設計書 §8.1）。正解の配置と頂点座標を `expected.json` に持つ。

### 米子コンベンションセンター（ビッグシップ）施設図面（`real/bigship/`）

実在する建物の平面図。施設の利用者向けに公開されている DXF。

- 取得元: <https://www.bigship.or.jp/bigship/organizer/3/>
- 利用条件: ページに再配布・二次利用の条件の記載が無い。**手元での検証にだけ使い、リポジトリにも公開デモにも入れない**

```bash
mkdir -p testdata/real/bigship && cd testdata/real/bigship
B=https://www.bigship.or.jp
curl -o 12f-heimenhall.dxf  $B/user/filer_public/43/96/4396965d-8084-4a77-8b97-d4cc0e72bebe/12f-heimenhall.dxf
curl -o entoransurobi.dxf   $B/user/filer_public/3f/f3/3ff35c89-ffde-48cb-a391-79e907938f21/entoransurobi.dxf
curl -o stage_100_1.dxf     $B/user/filer_public/7f/2b/7f2b1574-5a67-4072-8271-dabfaa857516/100_1.dxf
```

| ファイル | 図面 | `$ACADVER` | `$DWGCODEPAGE` | `$INSUNITS` | 特徴 |
|---|---|---|---|---|---|
| `12f-heimenhall.dxf` | 全体平面図 1F（客席）・2F | AC1009（R12） | なし | なし | レイヤ名が `_0-0_0` 形式（Jw_cad から書き出したと思われる）。単位情報なし。範囲 0〜168000 × 0〜118800 |
| `entoransurobi.dxf` | エントランスロビー（ホール棟 1F） | AC1015（2000） | ANSI_932 | 4（mm） | 日本語は cp932。MTEXT・POLYLINE あり |
| `stage_100_1.dxf` | 多目的ホール舞台平面図 1/100 | AC1024（2010） | ANSI_932 | 4（mm） | R2010 なので文字列は UTF-8 |

3 ファイルで DXF の世代・文字コード・単位の有無がすべて違う。

### Autodesk の AutoCAD サンプル図面（`real/autodesk/`）

本物の AutoCAD が書き出した DWG。建築系のサンプル（Architectural Example、`home_floor_plan.dwg` など）を使う。

- 取得元: <https://autodesk.com/autocad-samples>、<https://autodesk.com/autocadlt-samples>（ブラウザで手動ダウンロード）
- 利用条件: Autodesk のサンプルとしての提供。**手元での検証にだけ使い、リポジトリにも公開デモにも入れない**

## 測量図面（座標付き）

### 大阪市地形図 DXF（`real/osaka/`）

- 取得元: <https://www.geospatial.jp/ckan/dataset/r07-_dm-dxf-aprx-pdf>（「04_地形図DXFデータ」約 300 MB）
- 利用条件: 公共測量成果（大阪市・道路管理センター）。GIS の背景図に使うなどの複製には測量法第 43 条・第 44 条の承認が必要な場合がある。**手元での変換検証にだけ使う**

### 神奈川県 3 次元点群の等高線 DXF（`real/kanagawa/`）

- 取得元: <https://www.geospatial.jp/ckan/dataset/kanagawa-2024-pointcloud>（「等高線データ(DXF)」）
- 利用条件: CC BY 4.0。公開デモ・CI のテストに使ってよい（出典を明記する）
