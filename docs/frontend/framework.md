# ドキュメントのスコープ
本ドキュメントは以下をスコープとする。

- フロントエンドSPAの記述言語の選定
- フロントエンドSPAにおいて利用するフレームワークの選定

# フロントエンドSPAにおける利用技術
## ビルドツール
ビルドツールとしては、[Vite](https://ja.vite.dev/)を利用する。
バージョンは最新のv8系列を使用する。

## SPAフレームワーク
SPAフレームワークとしては[React](https://ja.react.dev/)を利用する。
バージョンは最新のv18系列を使用する。

## 記述言語
記述言語としては[TypeScript](https://www.typescriptlang.org/)を利用する。
バージョンは最新のv7系列を使用する。

## UIライブラリ
[Chakra UI](https://chakra-ui.com/)を利用する。
バージョンは最新のv3系列を使用する。

## アイコンセット
[Lucide](https://lucide.dev/)を使用する。
公式で[Reactパッケージ](https://lucide.dev/guide/react/)が用意されているため実際にはそれを利用する。

## OAuth 2.0
OAuth 2.0 Authorization Code Flow with PKCE及びOpenID Connectの処理には、
[react-oidc-context](https://github.com/authts/react-oidc-context)と、その基盤である[oidc-client-ts](https://github.com/authts/oidc-client-ts)を使用する。

認証処理はこれらのライブラリに委譲し、独自実装しない。
