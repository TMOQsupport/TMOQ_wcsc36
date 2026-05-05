# TMOQ_wcsc36

TMOQ(特大もっきゅ) Wcsc36版 - Rust言語による将棋エンジン

## 概要

TMOQ_wcsc36はRust言語で完全に新規実装された将棋エンジンです。やねうら王nano-plusのアルゴリズムを参考に、基本データ構造から探索アルゴリズムまで、全てをスクラッチで構築しました。

Claude（Anthropic製LLM）とのペアプログラミングにより開発されました。

## 大会成績

- **第36回世界コンピュータ将棋選手権（WCSC36）**: 一次予選 37チーム中34位 1勝1不戦勝6敗（2026年5月）

## 技術的特徴

- **言語**: Rust（完全新規実装）
- **データ構造**: Bitboard、Position、Move全て自作
- **探索**: やねうら王nano-plus準拠（αβ + LMR + Futility Pruning）
- **評価関数**: 駒得評価 + パラメータ最適化システム
- **定跡**: YaneuraOu DB2016形式対応
- **コード行数**: 約3,800行

## 主な実装内容

### 探索アルゴリズム

- αβ探索 + 置換表
- 反復深化（Iterative Deepening）
- Late Move Reduction (LMR)
- Futility Pruning
- Null Move Pruning
- History Heuristic
- Killer Move
- Counter Move
- 静止探索（Quiescence Search）

### 評価関数

- 駒得評価（Material Evaluation）
- 位置評価（Piece-Square Tables）
- 玉の安全度評価
- 手番の価値
- パラメータ最適化システム（27個のパラメータ）

### その他の機能

- USIプロトコル完全対応
- 定跡データベース（YaneuraOu DB2016形式）
- パラメータ外部化（JSON）
- Pythonによる自動パラメータ調整

## ビルド方法

### 必要なもの

- Rust（最新安定版）
- Cargo

### ビルド

```bash
cargo build --release --bin engine
```

実行ファイルは `target/release/engine` に生成されます。

## 使い方

### USIプロトコルで起動

```bash
./target/release/engine
```

### 将棋GUIで使用

将棋所や将棋GUIなどのUSI対応GUIで使用できます。

エンジン登録時：
- エンジンのパス: `target/release/engine`（または `engine.exe`）
- 作業フォルダ: プロジェクトルート

## パラメータ調整

評価関数のパラメータは `params.json` で調整できます。

### パラメータファイルの配置

```
プロジェクトルート/params.json
target/release/params.json
```

### パラメータ最適化

Pythonスクリプトを使用した自動調整：

```bash
python tune_params.py
```

詳細は `docs/PARAMETER_TUNING_GUIDE.md` を参照。

## プロジェクト構成

```
TMOQ_wcsc36/
├── src/
│   ├── lib.rs              # ライブラリルート
│   ├── types.rs            # 基本型定義
│   ├── bitboard.rs         # Bitboard実装
│   ├── piece.rs            # 駒の定義
│   ├── hand.rs             # 持ち駒管理
│   ├── move.rs             # 指し手表現
│   ├── position.rs         # 局面管理・評価関数
│   ├── eval_params.rs      # 評価パラメータ
│   ├── usi.rs              # 探索エンジン
│   ├── book.rs             # 定跡管理
│   └── bin/
│       └── engine.rs       # エンジン本体
├── tune_params.py          # パラメータ調整スクリプト
├── test_single_param.py    # 単一パラメータテスト
├── Cargo.toml              # プロジェクト設定
├── params.json             # パラメータファイル
└── README.md               # このファイル
```

## 開発の経緯

### Phase 1-6: 基礎構築（2026年1月）

Python版からスタートし、Rust版への完全移植を実施。

### Phase 7-12: やねうら王から学ぶ（2026年2月）

やねうら王ブログ記事を研究し、評価関数を強化。

### Phase 13-14: nano-plusへの進化（2026年3月）

やねうら王nano/nano-plusのソースコードを研究し、探索アルゴリズムを実装。

### Phase 15-16: パラメータ最適化（2026年4月）

データ駆動のパラメータ最適化システムを構築。

詳細は 下記を参照。
https://www.apply.computer-shogi.org/wcsc36/appeal/TMOQ/TMOQ_appeal_20260412.pdf

## 使用ライブラリ

- Rust標準ライブラリ
- serde/serde_json（パラメータ管理用）

外部の将棋ライブラリには依存せず、完全自作実装です。

## ライセンス

MIT License

Copyright (c) 2026 TMOQ_wcsc36 Contributors

詳細は [LICENSE](LICENSE) ファイルを参照。

## 謝辞

- やねうら王開発者の磯崎元洋氏
- 情報を公開してくださった多くの方々
- Claude（Anthropic製LLM）

## 作者

開発者: TMOQ（特大もっきゅ）

## リンク

- [やねうら王公式](http://yaneuraou.yaneu.com/)

## 参考資料

- やねうら王ブログ「コンピュータ将棋で世界を獲る」
- やねうら王nano/nano-plusソースコード

---

**Built with Rust 🦀 and Claude 🤖**
