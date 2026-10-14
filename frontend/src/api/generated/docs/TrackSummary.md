
# TrackSummary

収録曲一覧用の楽曲情報

## Properties

Name | Type
------------ | -------------
`id` | string
`releaseId` | string
`title` | string
`discNumber` | number
`trackNumber` | number
`durationMs` | number
`artistCredits` | [Array&lt;ArtistCredit&gt;](ArtistCredit.md)

## Example

```typescript
import type { TrackSummary } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "releaseId": null,
  "title": null,
  "discNumber": null,
  "trackNumber": null,
  "durationMs": null,
  "artistCredits": null,
} satisfies TrackSummary

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as TrackSummary
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


