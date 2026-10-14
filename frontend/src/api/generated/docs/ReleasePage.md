
# ReleasePage

カーソル方式のリリース一覧。 最終ページのitemsは空配列でもよい。 nextCursorを含む場合、itemsには1件以上含まれる。 

## Properties

Name | Type
------------ | -------------
`items` | [Array&lt;ReleaseSummary&gt;](ReleaseSummary.md)
`nextCursor` | string

## Example

```typescript
import type { ReleasePage } from ''

// TODO: Update the object below with actual values
const example = {
  "items": null,
  "nextCursor": null,
} satisfies ReleasePage

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as ReleasePage
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


