# ContentAccessApi

All URIs are relative to *http://localhost:3000/api/v0*

| Method | HTTP request | Description |
|------------- | ------------- | -------------|
| [**clearContentAccess**](ContentAccessApi.md#clearcontentaccess) | **DELETE** /content-access | コンテンツアクセス設定の解除 |
| [**configureContentAccess**](ContentAccessApi.md#configurecontentaccess) | **POST** /content-access | コンテンツアクセスの設定 |



## clearContentAccess

> clearContentAccess()

コンテンツアクセス設定の解除

コンテンツ配信エンドポイントへアクセスするために設定された認可情報をクライアントから削除する。  認可情報が設定される構成では、レスポンスの&#x60;Set-Cookie&#x60;ヘッダーによって削除する。 Cookieの具体的な内容は実装の詳細であり、クライアントは解釈または直接操作してはならない。

### Example

```ts
import {
  Configuration,
  ContentAccessApi,
} from '';
import type { ClearContentAccessRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const config = new Configuration({
    // Configure HTTP bearer authorization: BearerAuth
    accessToken: "YOUR BEARER TOKEN",
  });
  const api = new ContentAccessApi(config);

  try {
    const data = await api.clearContentAccess();
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters

This endpoint does not need any parameter.

### Return type

`void` (Empty response body)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/problem+json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **401** | 認証有効時、Bearerアクセストークンが未指定または不正である |  * WWW-Authenticate -  <br>  |
| **204** | コンテンツアクセス設定の解除が完了した |  * Set-Cookie - 配信アクセス用Cookieを削除する。 <br>  |
| **500** | サーバー内部で予期しないエラーが発生した |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## configureContentAccess

> ContentAccessConfiguration configureContentAccess()

コンテンツアクセスの設定

コンテンツ配信エンドポイントへアクセスするために必要な認可情報をクライアントに設定する。  認可情報が必要な構成では、レスポンスの&#x60;Set-Cookie&#x60;ヘッダーによって設定する。 Cookieの具体的な内容は実装の詳細であり、クライアントは解釈または直接操作してはならない。  レスポンスに&#x60;invalidAfter&#x60;が含まれる場合、その時刻以降は設定された認可情報の有効性を保証しない。 クライアントは&#x60;invalidAfter&#x60;より前の任意の時刻に本APIを再度実行し、コンテンツアクセス設定を更新できる。

### Example

```ts
import {
  Configuration,
  ContentAccessApi,
} from '';
import type { ConfigureContentAccessRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const config = new Configuration({
    // Configure HTTP bearer authorization: BearerAuth
    accessToken: "YOUR BEARER TOKEN",
  });
  const api = new ContentAccessApi(config);

  try {
    const data = await api.configureContentAccess();
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters

This endpoint does not need any parameter.

### Return type

[**ContentAccessConfiguration**](ContentAccessConfiguration.md)

### Authorization

[BearerAuth](../README.md#BearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`, `application/problem+json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **401** | 認証有効時、Bearerアクセストークンが未指定または不正である |  * WWW-Authenticate -  <br>  |
| **200** | コンテンツアクセスに必要な設定が完了した |  * Set-Cookie - 配信アクセス用Cookie。複数のSet-Cookieヘッダーで返す。 <br>  |
| **500** | サーバー内部で予期しないエラーが発生した |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)

