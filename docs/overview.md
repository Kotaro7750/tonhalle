# ドキュメントのスコープ
本ドキュメントは以下をスコープとする。

- 本リポジトリの概要
- 本リポジトリにおける設計ドキュメントの構造

# 本リポジトリの概要
本リポジトリは、音楽ファイルを再生・管理するためのアプリケーションtonhalleの開発リポジトリである。

# 文書の構造
本リポジトリにおける設計ドキュメント・コーディングガイドライン等のドキュメント群は、`docs`ディレクトリ配下にマークダウンファイルとして格納する。
`docs`直下には、成果物の個別実装によらない仕様・要件を配置する。
各成果物固有のソフトウェア設計やコーディング規約は、対応するサブディレクトリに配置する。

ファイル名で単語を区切る場合は、アンダーバー（_）を使用する。 他のファイルを参照する場合は相対パスによるMarkdownリンクを使用し、リンクテキストには拡張子を含むファイル名を記載する。

サブディレクトリを作成する場合は、その直下にoverview.mdを配置する。 本ドキュメントに各サブディレクトリについて記載する内容は、そのサブディレクトリが扱うドキュメントの役割と、そのサブディレクトリのoverview.mdへのリンクのみとする。 サブディレクトリ配下の個別のドキュメントは本ドキュメントには記載せず、各サブディレクトリのoverview.mdから確認する。

- [overview.md](./overview.md): この索引と設計の前提
- [documentation.md](./documentation.md): ドキュメントの記載ガイドライン
- [api.md](./api.md): API 契約の作成方針
- [architecture.md](./architecture.md): システム構成と各構成要素の責務
- [artifacts.md](./artifacts.md): 成果物の配置
- [backend](./backend/): バックエンドAPIサーバーに関する設計ドキュメントを配置するドキュメント階層。配下のドキュメント構造は[backend/overview.md](./backend/overview.md)に記載
- [domain.md](./domain.md): ドメインモデルの定義
- [frontend/overview.md](./frontend/overview.md): フロントエンドSPAに関する設計ドキュメントを配置するドキュメント階層。配下のドキュメント構造は[frontend/overview.md](./frontend/overview.md)に記載
- [music_data.md](./music_data.md): 実際の音源データ
- [music_data_storage.md](./music_data_storage.md): 音源データストレージの要件
- [roadmap.md](./roadmap.md): 本プロジェクトにおける今後の開発計画を記述する
- [versioning.md](./versioning.md): 成果物ごとのバージョン管理方針

