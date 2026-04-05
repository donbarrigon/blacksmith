import type { NameConvention } from "./NameConvention"

export interface Args {
  targets: string[]
  name: NameConvention | undefined
  flags: string[]
}
