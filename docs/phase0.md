# Phase 0 の調査結果

実施: 2026-10-12 / 設計書 [§8 Phase 0](design.md#8-フェーズ) の 4 項目

## 1. acadrust で DXF の日本語を読めるか

acadrust 0.6.4 の `DxfReader` で、ビッグシップの施設図面 DXF 3 枚を読んだ（試験用の小さなプログラムで、TEXT / MTEXT の文字列を数えた）。

| DXF | acadrust（既定） | 参考: GDAL 3.13.3 |
|---|---|---|
| R2000・`$DWGCODEPAGE = ANSI_932` | 正しい（52 中 34 が日本語、化けなし） | 正しい |
| R2010・UTF-8（ヘッダは `ANSI_932`） | **正しい**（94 中 94）。AC1021（2007）以降はヘッダに関係なく UTF-8 として読む | 文字化け |
| R12・`$DWGCODEPAGE` なし | **文字化け**（`ANSI_1252` として読む。201 中 98 に Latin-1 の文字） | GeoJSON が壊れる |
| 同上、`default_encoding = "ANSI_932"` | **正しい**（201 中 177 が日本語） | ― |

- R12 は `DxfReaderConfiguration::default_encoding` を指定すれば正しく読める
- ただし指定できるのは DXF の表記（`ANSI_932`）だけで、`cp932` / `shift_jis` / `windows-31j` は**黙って無視され `ANSI_1252` で読まれる**。エラーにならないので気づきにくい（acadrust への issue の候補）
- MTEXT の値には書式コードが残る（例: `{\fメイリオ|b0|i0|c128|p0;天井反射板（１）}`）。dwg2geo は `text` に書式を除いた値、`text_raw` に元の値を出す

**cad2geo への示唆**: `$DWGCODEPAGE` の無い R12 の DXF は、文字列のバイト列が cp932 として妥当で日本語の文字を含むかを見て、そうなら `ANSI_932` を指定して読み直す。Jw_cad 由来の図面はこれで読める。

## 2. `$MEASUREMENT` の読み違い

acadrust 0.6.4 の `DwgReader` で、ODA が書いた DWG 6 ファイルを読んだ。

| DWG | acadrust 0.6.4 | dwg2geo 0.2.5（acadrust 0.5.1） | 正解（ODA で DXF に戻した値） |
|---|---|---|---|
| エントランスロビー（AC1032 / AC1015） | **1（メートル法）** | english | 1 |
| 舞台平面図（AC1032 / AC1015） | **1** | english | 1 |
| 全体平面図（AC1032 / AC1015） | 0 | english | 0（元の R12 に記録なし） |

acadrust 0.6.4 は正しく読む。dwg2geo が acadrust 0.5.1 に固定されていることが原因と見てよい。**dwg2geo が acadrust を 0.6 系に上げれば直る**（API の互換性は未確認）。

## 3. npm の WASM 版

npm の `dwg2geo@0.2.5`（WASM 1.26 MB）を Node 22 で動かした。

| DWG | 図形数 | 所要 | CLI 版との比較 |
|---|---:|---:|---|
| 全体平面図（AC1032） | 24,664 | 1.4 秒 | 図形数・日本語・外接矩形が一致 |
| エントランスロビー（AC1032） | 2,595 | 0.05 秒 | 一致（WIPEOUT 3 件を報告） |
| 舞台平面図（AC1015） | 1,533 | 0.04 秒 | 一致 |

- API は `convert(bytes, polygonize_closed, curve_tolerance)` 1 つ。図面座標の GeoJSON 文字列とレポート（`skipped`、`bbox`、`geodata`、`crs_text_hints` など）を返す
- ブラウザ版はこれで変換できる。座標系変換は呼び出し側の責任（作者の想定は proj4js）

## 4. 上流の様子

| | dwg2geo | acadrust |
|---|---|---|
| 作者 | André Leite（`milkway`）1 名 | hakanaktt |
| 作成 / 最終更新 | 2026-07 / 2026-09-30 | ― / 2026-10-11 |
| リリース | v0.2.0〜0.2.5（7〜9 月に 6 回） | 0.6.4 |
| issue / PR | **これまで 0 件** | 未解決の issue 12 件 |
| スター | 1 | 105 |

dwg2geo は活発だが外部からの貢献の前例が無く、受け入れてもらえるかは送ってみないと分からない。小さく確実なもの（acadrust の更新）から送るのがよい。

## 5. 上流に送るものの候補（順番の案）

| 順 | 送り先 | 種類 | 内容 | 根拠 |
|---|---|---|---|---|
| 1 | dwg2geo | PR | acadrust を 0.6 系に上げる（`$MEASUREMENT` が直る） | §2 |
| 2 | acadrust | issue | `default_encoding` に解釈できない名前を渡すと黙って `ANSI_1252` になる。別名（`cp932` など）を受け付けるか、エラーにしてほしい | §1 |
| 3 | dwg2geo | PR | DXF の入力（`read_stream()` で DXF を判別し acadrust の `DxfReader` に渡す。`default_encoding` を指定できるようにする） | §1 |
| 4 | dwg2geo | issue | 位置合わせで縮尺を固定する選択肢、対応点を経緯度で渡したときに投影座標系で解く処理 | 評価 §2.7 |
| 5 | dwg2geo | issue | 文字の揃え位置の属性、位置合わせを文字の回転角・文字高に反映 | 評価 §2.6・§2.7 |
