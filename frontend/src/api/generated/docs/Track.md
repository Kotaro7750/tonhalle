
# Track

楽曲情報と唯一のオリジナル音源

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
`originalAudioFile` | [AudioFile](AudioFile.md)

## Example

```typescript
import type { Track } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "releaseId": null,
  "title": null,
  "discNumber": null,
  "trackNumber": null,
  "durationMs": null,
  "artistCredits": null,
  "originalAudioFile": null,
} satisfies Track

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as Track
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


