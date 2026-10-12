# 評価: dwg2geo v0.2.5

実施: 2026-10-12 / 対象: [dwg2geo](https://github.com/milkway/dwg2geo) v0.2.5（Windows 用ビルド済みバイナリ、`native-backend` 入り、`native-reproject` なし）

設計書「先行例: dwg2geo」の判断材料として、実在する建築図面で dwg2geo を試した。

## 1. 入力

米子コンベンションセンター（ビッグシップ）の施設図面 DXF 3 枚（取得方法は [testdata/README.md](../testdata/README.md)）を、ODA File Converter 27.9.0 で DWG に変換した 6 ファイル。

| 元の DXF | 図面 | `$ACADVER` | 文字コード | `$INSUNITS` | 変換した DWG |
|---|---|---|---|---|---|
| `12f-heimenhall.dxf` | 全体平面図 1F・2F | AC1009（R12） | 記録なし（cp932） | なし | AC1032 / AC1015 |
| `entoransurobi.dxf` | エントランスロビー | AC1015（2000） | ANSI_932 | 4（mm） | AC1032 / AC1015 |
| `stage_100_1.dxf` | 多目的ホール舞台平面図 | AC1024（2010） | UTF-8 | 4（mm） | AC1032 / AC1015 |

比較のため、元の DXF を GDAL 3.13.3 の `ogr2ogr` でも GeoJSON にした。

```bash
dwg2geo inspect <dwg> --json
dwg2geo convert <dwg> -o <out.geojson> --backend native --allow-local-coordinates
```

## 2. 結果

### 2.1 読み込み

6 ファイルとも strict モードで読めた。エラー 0、未知のエンティティ 0。

| 図面 | エンティティ（モデル空間） | 内訳 |
|---|---:|---|
| 全体平面図 | 24,660 | LINE 20,982 / ARC 3,277 / TEXT 201 / CIRCLE 199 / INSERT 1 |
| エントランスロビー | 2,597 | LINE 2,139 / ARC 351 / MTEXT 52 / CIRCLE 38 / POLYLINE 6 / HATCH 5 / WIPEOUT 3 / SOLID 2 / INSERT 1 |
| 舞台平面図 | 1,533 | LINE 882 / ARC 556 / MTEXT 94 / LWPOLYLINE 1 |

**AC1032 と AC1015 の出力は、ハンドル（図面内 ID）を除いて完全に一致した。** 3 図面とも、ジオメトリと属性の組がすべて同じだった。

### 2.2 変換

- 全エンティティの行き先がレポートで説明されている（`accounting.unaccounted = 0`）
- 未対応は WIPEOUT 3 件のみで、理由とハンドルつきでレポートに出る
- ブロック展開、BYLAYER の色・線種の解決、文字の高さ・回転角が属性に入る

### 2.3 日本語

3 図面とも、TEXT / MTEXT の日本語が正しく出た（`電話コーナー`、`多目的ホール`、`天井反射板（１）` など）。

同じ図面を GDAL で DXF から読むと、3 枚中 2 枚で日本語が壊れた。

| 元の DXF | GDAL 3.13.3（DXF を直接） | dwg2geo（ODA で DWG にしたもの） |
|---|---|---|
| R12・文字コード記録なし | cp932 のバイト列をそのまま出力し、**GeoJSON として読めないファイル**になる | 正しい |
| 2000・cp932 | 正しい | 正しい |
| 2010・UTF-8（ヘッダは ANSI_932） | ヘッダを信じて cp932 として読み、`螟ｩ莠募渚...` と**文字化け** | 正しい |

ただし dwg2geo が正しく読めたのは、**ODA が DXF → DWG の変換時に文字コードを正規化したおかげ**でもある（ODA は R12 の DXF に `ANSI_932` を補った）。dwg2geo は DXF を直接読めないので（§2.5）、DXF の文字コード判定は評価できていない。

細かい差として、cp932 の `0x817C`（長音の代わりに使われがちな「－」）を、GDAL は U+2212（−）、dwg2geo は U+FF0D（－）にしている。Windows で作られた図面なら U+FF0D が妥当。

### 2.4 GDAL との図形の差（エントランスロビー）

| | 件数 | 範囲（Y の最小） |
|---|---:|---:|
| GDAL | 2,597 | −107,110 |
| dwg2geo | 2,595 | 71,704 |

差は WIPEOUT 3 件。GDAL は WIPEOUT を座標の壊れたポリゴンとして出しており、建物から 100 m 以上南に伸びている。dwg2geo は WIPEOUT を変換せず、レポートで報告している。**dwg2geo の扱いのほうが正しい。**

### 2.5 問題点

| 項目 | 内容 |
|---|---|
| **DXF を読めない** | `.dxf` を渡すと「Unsupported CAD version」で失敗する。DWG 専用 |
| **`$MEASUREMENT` の読み違い** | mm 図面（`$MEASUREMENT = 1`、メートル法）を `english` と報告する。ODA で DWG → DXF に戻すと 1 が入っているので、DWG 側は正しい。dwg2geo は acadrust 0.5.1 に固定しており、0.6.4 で追加された「AcDb:Template セクションから `$MEASUREMENT` を読む」処理が無いためと思われる（未検証）。座標系変換では `$INSUNITS` と `$MEASUREMENT` の一致を見て単位を決めるので、mm 図面で毎回 `--source-units` が要ることになる |
| **ビルド済みバイナリに座標系変換が無い** | `--source-crs` には `native-reproject` 機能と PROJ 9.6 以上が必要。WASM 版でも使えない見込み |

## 3. 未評価

- **対応点による位置合わせ**（`--control-point`）: 図面上の点と地図上の点の対応を決める必要があり、今回は試していない。ビッグシップの外形と地図（地理院の写真、OSM / PLATEAU の建物）で対応点を取って試す
- **座標系変換**（`--source-crs`）: ビルド済みバイナリに無い。C++ ビルドツールを入れて `native-reproject` 付きでビルドしてから
- **本物の AutoCAD が書いた DWG**: 今回の DWG はすべて ODA が書いたもの。Autodesk のサンプルで試す
- **WASM 版（npm の `dwg2geo`）**

## 4. 所見

- **DWG の読み込みと変換の品質は高い。** 日本の実図面で、形式（2000 / 2018）による差がなく、日本語も正しく、未対応は明示される。GDAL より正確な点もある（WIPEOUT）
- **足りないのは設計書で挙げたとおりの部分。** DXF の入力、日本固有の対応（平面直角座標系、DXF の文字コード判定）、ブラウザで使える座標系変換、地図上で位置を合わせる UI
- DXF の文字コード判定は GDAL も間違える（§2.3）ので、ここは独自に作る価値がある

位置合わせ（§3）を試してから、cad2geo の方針（dwg2geo を土台にするか）を決める。
