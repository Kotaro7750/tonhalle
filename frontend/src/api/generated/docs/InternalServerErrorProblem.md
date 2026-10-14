
# InternalServerErrorProblem

サーバー内部で予期しないエラーが発生した

## Properties

Name | Type
------------ | -------------
`type` | string
`title` | string
`status` | number
`detail` | string
`instance` | string

## Example

```typescript
import type { InternalServerErrorProblem } from ''

// TODO: Update the object below with actual values
const example = {
  "type": null,
  "title": null,
  "status": null,
  "detail": null,
  "instance": null,
} satisfies InternalServerErrorProblem

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as InternalServerErrorProblem
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


