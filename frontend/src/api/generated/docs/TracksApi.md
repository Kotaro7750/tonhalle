# TracksApi

All URIs are relative to *http://localhost:3000/api/v0*

| Method | HTTP request | Description |
|------------- | ------------- | -------------|
| [**getTrack**](TracksApi.md#gettrack) | **GET** /tracks/{trackId} | トラック詳細の取得 |



## getTrack

> Track getTrack(trackId)

トラック詳細の取得

指定したトラックの楽曲情報とオリジナル音源の情報を取得する。

### Example

```ts
import {
  Configuration,
  TracksApi,
} from '';
import type { GetTrackRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new TracksApi();

  const body = {
    // string | Trackの識別子
    trackId: trackId_example,
  } satisfies GetTrackRequest;

  try {
    const data = await api.getTrack(body);
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
| **trackId** | `string` | Trackの識別子 | [Defaults to `undefined`] |

### Return type

[**Track**](Track.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`, `application/problem+json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | トラック詳細の取得に成功した |  -  |
| **400** | 識別子の形式、カーソル、またはlimitが不正である |  -  |
| **404** | 指定したリリースまたはトラックが存在しない |  -  |
| **500** | サーバー内部で予期しないエラーが発生した |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)

