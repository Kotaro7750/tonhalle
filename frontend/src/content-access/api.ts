import { api } from '../api/client'

let configuring: ReturnType<typeof api.configureContentAccess> | undefined

export function configureContentAccess() {
  configuring ??= api.configureContentAccess().finally(() => { configuring = undefined })
  return configuring
}

export const clearContentAccess = api.clearContentAccess
