# ReleasesApi

All URIs are relative to *http://localhost:3000/api/v0*

| Method | HTTP request | Description |
|------------- | ------------- | -------------|
| [**getRelease**](ReleasesApi.md#getrelease) | **GET** /releases/{releaseId} | リリース詳細の取得 |
| [**listReleases**](ReleasesApi.md#listreleases) | **GET** /releases | リリース一覧の取得 |



## getRelease

> Release getRelease(releaseId)

リリース詳細の取得

指定したリリースの情報と全収録曲を取得する。

### Example

```ts
import {
  Configuration,
  ReleasesApi,
} from '';
import type { GetReleaseRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new ReleasesApi();

  const body = {
    // string | Releaseの識別子
    releaseId: releaseId_example,
  } satisfies GetReleaseRequest;

  try {
    const data = await api.getRelease(body);
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
| **releaseId** | `string` | Releaseの識別子 | [Defaults to `undefined`] |

### Return type

[**Release**](Release.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`, `application/problem+json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | リリース詳細の取得に成功した |  -  |
| **400** | 識別子の形式、カーソル、またはlimitが不正である |  -  |
| **404** | 指定したリリースまたはトラックが存在しない |  -  |
| **500** | サーバー内部で予期しないエラーが発生した |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listReleases

> ReleasePage listReleases(cursor, limit)

リリース一覧の取得

リリース概要をidの昇順でページごとに取得する。 各ページはリクエスト時点の一覧を対象とし、一連の取得を通じたスナップショットは保証しない。 

### Example

```ts
import {
  Configuration,
  ReleasesApi,
} from '';
import type { ListReleasesRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new ReleasesApi();

  const body = {
    // string | 取得開始位置を指定するカーソル。 初回または先頭から取得し直す場合は省略する。 続きの取得では前のレスポンスのnextCursorをそのまま指定し、内容を解釈してはならない。  (optional)
    cursor: cursor_example,
    // number | 1ページの最大件数。 継続リクエストで変更してもよい  (optional)
    limit: 56,
  } satisfies ListReleasesRequest;

  try {
    const data = await api.listReleases(body);
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
| **cursor** | `string` | 取得開始位置を指定するカーソル。 初回または先頭から取得し直す場合は省略する。 続きの取得では前のレスポンスのnextCursorをそのまま指定し、内容を解釈してはならない。  | [Optional] [Defaults to `undefined`] |
| **limit** | `number` | 1ページの最大件数。 継続リクエストで変更してもよい  | [Optional] [Defaults to `30`] |

### Return type

[**ReleasePage**](ReleasePage.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`, `application/problem+json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | リリース一覧の取得に成功した |  -  |
| **400** | 識別子の形式、カーソル、またはlimitが不正である |  -  |
| **500** | サーバー内部で予期しないエラーが発生した |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)

