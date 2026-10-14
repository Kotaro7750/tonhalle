
# ReleaseSummary

一覧表示用のリリース概要

## Properties

Name | Type
------------ | -------------
`id` | string
`title` | string
`releaseDate` | string
`artistCredits` | [Array&lt;ArtistCredit&gt;](ArtistCredit.md)

## Example

```typescript
import type { ReleaseSummary } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "title": null,
  "releaseDate": null,
  "artistCredits": null,
} satisfies ReleaseSummary

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as ReleaseSummary
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


