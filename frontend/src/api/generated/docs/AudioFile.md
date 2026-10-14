
# AudioFile

オリジナル音源の情報。

## Properties

Name | Type
------------ | -------------
`id` | string
`mediaType` | string
`playbackUrl` | string
`sampleRate` | number
`bitDepth` | number
`channels` | number
`bitrate` | number

## Example

```typescript
import type { AudioFile } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "mediaType": null,
  "playbackUrl": null,
  "sampleRate": null,
  "bitDepth": null,
  "channels": null,
  "bitrate": null,
} satisfies AudioFile

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as AudioFile
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


