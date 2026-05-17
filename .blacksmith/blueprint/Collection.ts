import type { EngineType, Field, Index, Migration } from "../types/migration"

export class Blueprint {
  private collection: string
  private fields: Field[]
  private indexes: Index[]
  private validationLevel?: "strict" | "moderate" | "off"
  private validationAction?: "error" | "warn"
  private additionalProperties?: boolean
  private engine?: EngineType
  private _dropCollection?: boolean
  private _dropField?: string[]
  private _dropIndex?: string[][]
  private _alterField?: Field[]
  private _alterIndex?: Index[]

  constructor(name: string) {
    this.collection = name
    this.fields = []
    this.indexes = []
  }

  dropCollection(): void {
    this._dropCollection = true
  }

  dropField(name: string): this {
    if (!this._dropField) this._dropField = []
    this._dropField.push(name)
    return this
  }

  dropIndex(idx: string | string[]): this {
    if (!this._dropIndex) this._dropIndex = []
    if (typeof idx === "string") {
      this._dropIndex.push([idx])
    } else {
      this._dropIndex.push(idx)
    }
    return this
  }
}
