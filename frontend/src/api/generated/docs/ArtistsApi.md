# ArtistsApi

All URIs are relative to *http://localhost:3000/api/v0*

| Method | HTTP request | Description |
|------------- | ------------- | -------------|
| [**getArtist**](ArtistsApi.md#getartist) | **GET** /artists/{artistId} | アーティスト詳細の取得 |
| [**listArtists**](ArtistsApi.md#listartists) | **GET** /artists | アーティスト一覧の取得 |



## getArtist

> Artist getArtist(artistId)

アーティスト詳細の取得

指定したアーティストの情報を取得する。 Releaseのアーティストクレジットに含まれるアーティストのみを対象とする。 指定したidがReleaseのアーティストクレジットに存在しない場合には404を返す。

### Example

```ts
import {
  Configuration,
  ArtistsApi,
} from '';
import type { GetArtistRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const config = new Configuration({
    // Configure HTTP bearer authorization: BearerAuth
    accessToken: "YOUR BEARER TOKEN",
  });
  const api = new ArtistsApi(config);

  const body = {
    // string | Artistの識別子
    artistId: artistId_example,
  } satisfies GetArtistRequest;

  try {
    const data = await api.getArtist(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **artistId** | `string` | Artistの識別子 | [Defaults to `undefined`] |

### Return type

[**Artist**](Artist.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`, `application/problem+json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **401** | 認証有効時、Bearerアクセストークンが未指定または不正である |  * WWW-Authenticate -  <br>  |
| **200** | アーティスト詳細の取得に成功した |  -  |
| **400** | 識別子の形式、カーソル、またはlimitが不正である |  -  |
| **404** | 指定したリソースが存在しない |  -  |
| **500** | サーバー内部で予期しないエラーが発生した |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listArtists

> ArtistPage listArtists(cursor, limit)

アーティスト一覧の取得

Releaseのアーティストクレジットに含まれるアーティストをidの昇順でページごとに取得する。 Trackのみにクレジットされたアーティストは含めない。 同じアーティストが複数のReleaseにクレジットされていても、同じidは1件として返す。 各ページはリクエスト時点の一覧を対象とし、一連の取得を通じたスナップショットは保証しない。

### Example

```ts
import {
  Configuration,
  ArtistsApi,
} from '';
import type { ListArtistsRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const config = new Configuration({
    // Configure HTTP bearer authorization: BearerAuth
    accessToken: "YOUR BEARER TOKEN",
  });
  const api = new ArtistsApi(config);

  const body = {
    // string | 取得開始位置を指定するカーソル。 初回または先頭から取得し直す場合は省略する。 続きの取得では前のレスポンスのnextCursorをそのまま指定し、内容を解釈してはならない。 カーソルは取得元の一覧APIと絞り込み条件に対応する。 継続リクエストでは同じAPIと絞り込み条件を使用すること。 異なるAPIや絞り込み条件のカーソルを指定した場合、期待した結果が得られない可能性がある。 絞り込み条件を変更する場合はカーソルを省略して先頭から取得し直す。  (optional)
    cursor: cursor_example,
    // number | 1ページの最大件数。 継続リクエストで変更してもよい  (optional)
    limit: 56,
  } satisfies ListArtistsRequest;

  try {
    const data = await api.listArtists(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **cursor** | `string` | 取得開始位置を指定するカーソル。 初回または先頭から取得し直す場合は省略する。 続きの取得では前のレスポンスのnextCursorをそのまま指定し、内容を解釈してはならない。 カーソルは取得元の一覧APIと絞り込み条件に対応する。 継続リクエストでは同じAPIと絞り込み条件を使用すること。 異なるAPIや絞り込み条件のカーソルを指定した場合、期待した結果が得られない可能性がある。 絞り込み条件を変更する場合はカーソルを省略して先頭から取得し直す。  | [Optional] [Defaults to `undefined`] |
| **limit** | `number` | 1ページの最大件数。 継続リクエストで変更してもよい  | [Optional] [Defaults to `30`] |

### Return type

[**ArtistPage**](ArtistPage.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`, `application/problem+json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **401** | 認証有効時、Bearerアクセストークンが未指定または不正である |  * WWW-Authenticate -  <br>  |
| **200** | アーティスト一覧の取得に成功した |  -  |
| **400** | 識別子の形式、カーソル、またはlimitが不正である |  -  |
| **500** | サーバー内部で予期しないエラーが発生した |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)

