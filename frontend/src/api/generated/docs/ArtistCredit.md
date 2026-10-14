
# ArtistCredit

表示順に並べるアーティストクレジットの1要素

## Properties

Name | Type
------------ | -------------
`artistId` | string
`name` | string
`joinPhrase` | string

## Example

```typescript
import type { ArtistCredit } from ''

// TODO: Update the object below with actual values
const example = {
  "artistId": null,
  "name": null,
  "joinPhrase": null,
} satisfies ArtistCredit

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as ArtistCredit
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


