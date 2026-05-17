// .blacksmith/blueprint/FieldBuilder.ts

import type { Field, FieldType, Index } from "../types/migration"

type StringFieldOptions = Pick<Field, "name" | "length" | "nullable" | "default" | "description">
type TextFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description">
type IntFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description" | "autoIncrement" | "unsigned">
type FloatFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description" | "unsigned">
type DecimalFieldOptions = Pick<
  Field,
  "name" | "nullable" | "default" | "description" | "precision" | "scale" | "unsigned"
>
type BoolFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description">
type DateFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description">
type EnumFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description" | "values">
type JsonFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description" | "jsonSchema">
type ArrayFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description" | "items">
type SimpleFieldOptions = Pick<Field, "name" | "nullable" | "default" | "description">

interface BlueprintRef {
  fields: Field[]
  indexes: Index[]
}

export class FieldBuilder {
  private field: Field
  private blueprint: BlueprintRef

  constructor(field: Field, blueprint: BlueprintRef) {
    this.field = field
    this.blueprint = blueprint
    this.blueprint.fields.push(this.field)
  }

  // ─── atributos generales ───────────────────────────────────────────────────

  nullable(): this {
    this.field.nullable = true
    return this
  }

  notNullable(): this {
    this.field.nullable = false
    return this
  }

  default(value: Field["default"]): this {
    this.field.default = value
    return this
  }

  description(value: string): this {
    this.field.description = value
    return this
  }

  // ─── atributos de string / char ───────────────────────────────────────────

  length(value: number): this {
    this.field.length = value
    return this
  }

  // ─── atributos numericos ──────────────────────────────────────────────────

  autoIncrement(): this {
    this.field.autoIncrement = true
    return this
  }

  unsigned(): this {
    this.field.unsigned = true
    return this
  }

  precision(value: number): this {
    this.field.precision = value
    return this
  }

  scale(value: number): this {
    this.field.scale = value
    return this
  }

  // ─── atributos de enum ────────────────────────────────────────────────────

  values(vals: string[]): this {
    this.field.values = vals
    return this
  }

  // ─── atributos de json ────────────────────────────────────────────────────

  jsonSchema(schema: Field[]): this {
    this.field.jsonSchema = schema
    return this
  }

  // ─── atributos de array ───────────────────────────────────────────────────

  items(items: Field["items"]): this {
    this.field.items = items
    return this
  }

  // ─── indices ──────────────────────────────────────────────────────────────

  unique(): this {
    this.blueprint.indexes.push({
      field: this.field.name as string,
      unique: true,
    })
    return this
  }

  primary(): this {
    this.blueprint.indexes.push({
      field: this.field.name as string,
      primary: true,
    })
    return this
  }

  index(): this {
    this.blueprint.indexes.push({
      field: this.field.name as string,
      type: "index",
    })
    return this
  }
}

// ─── Blueprint ────────────────────────────────────────────────────────────────

export class Blueprint {
  private collection: string
  fields: Field[]
  indexes: Index[]
  private validationLevel?: "strict" | "moderate" | "off"
  private validationAction?: "error" | "warn"
  private additionalProperties?: boolean
  private engine?: "InnoDB" | "MyISAM" | "MEMORY" | "CSV" | "ARCHIVE" | "BLACKHOLE" | "NDB"
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

  // ─── acciones ─────────────────────────────────────────────────────────────

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

  // ─── helpers ──────────────────────────────────────────────────────────────

  private addField(field: Field): FieldBuilder {
    return new FieldBuilder(field, this)
  }

  // ─── id ───────────────────────────────────────────────────────────────────

  id(t: "objectId" | "int64" | "int32" | "int16" | "int8" = "objectId"): void {
    new FieldBuilder({ name: "_id", type: t as FieldType, nullable: false, autoIncrement: true }, this)
    this.indexes.push({ field: "_id", primary: true })
  }

  // ─── tipos string ─────────────────────────────────────────────────────────

  string(name: string, length?: number): FieldBuilder
  string(options: StringFieldOptions): FieldBuilder
  string(nameOrOptions: string | StringFieldOptions, length = 255): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "string", length })
    }
    return this.addField({ length: 255, ...nameOrOptions, type: "string" })
  }

  char(name: string, length?: number): FieldBuilder
  char(options: StringFieldOptions): FieldBuilder
  char(nameOrOptions: string | StringFieldOptions, length = 1): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "char", length })
    }
    return this.addField({ length: 1, ...nameOrOptions, type: "char" })
  }

  text(name: string): FieldBuilder
  text(options: TextFieldOptions): FieldBuilder
  text(nameOrOptions: string | TextFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "text" })
    }
    return this.addField({ ...nameOrOptions, type: "text" })
  }

  // ─── tipos enteros con signo ──────────────────────────────────────────────

  int8(name: string): FieldBuilder
  int8(options: IntFieldOptions): FieldBuilder
  int8(nameOrOptions: string | IntFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "int8" })
    }
    return this.addField({ ...nameOrOptions, type: "int8" })
  }

  int16(name: string): FieldBuilder
  int16(options: IntFieldOptions): FieldBuilder
  int16(nameOrOptions: string | IntFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "int16" })
    }
    return this.addField({ ...nameOrOptions, type: "int16" })
  }

  int(name: string): FieldBuilder
  int(options: IntFieldOptions): FieldBuilder
  int(nameOrOptions: string | IntFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "int" })
    }
    return this.addField({ ...nameOrOptions, type: "int" })
  }

  int64(name: string): FieldBuilder
  int64(options: IntFieldOptions): FieldBuilder
  int64(nameOrOptions: string | IntFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "int64" })
    }
    return this.addField({ ...nameOrOptions, type: "int64" })
  }

  // ─── tipos enteros sin signo ──────────────────────────────────────────────

  uint8(name: string): FieldBuilder
  uint8(options: IntFieldOptions): FieldBuilder
  uint8(nameOrOptions: string | IntFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "uint8" })
    }
    return this.addField({ ...nameOrOptions, type: "uint8" })
  }

  uint16(name: string): FieldBuilder
  uint16(options: IntFieldOptions): FieldBuilder
  uint16(nameOrOptions: string | IntFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "uint16" })
    }
    return this.addField({ ...nameOrOptions, type: "uint16" })
  }

  uint(name: string): FieldBuilder
  uint(options: IntFieldOptions): FieldBuilder
  uint(nameOrOptions: string | IntFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "uint" })
    }
    return this.addField({ ...nameOrOptions, type: "uint" })
  }

  uint64(name: string): FieldBuilder
  uint64(options: IntFieldOptions): FieldBuilder
  uint64(nameOrOptions: string | IntFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "uint64" })
    }
    return this.addField({ ...nameOrOptions, type: "uint64" })
  }

  // ─── tipos decimales ──────────────────────────────────────────────────────

  float(name: string): FieldBuilder
  float(options: FloatFieldOptions): FieldBuilder
  float(nameOrOptions: string | FloatFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "float" })
    }
    return this.addField({ ...nameOrOptions, type: "float" })
  }

  double(name: string): FieldBuilder
  double(options: FloatFieldOptions): FieldBuilder
  double(nameOrOptions: string | FloatFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "double" })
    }
    return this.addField({ ...nameOrOptions, type: "double" })
  }

  decimal(name: string, precision?: number, scale?: number): FieldBuilder
  decimal(options: DecimalFieldOptions): FieldBuilder
  decimal(nameOrOptions: string | DecimalFieldOptions, precision = 10, scale = 2): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "decimal", precision, scale })
    }
    return this.addField({ precision: 10, scale: 2, ...nameOrOptions, type: "decimal" })
  }

  money(name: string): FieldBuilder
  money(options: DecimalFieldOptions): FieldBuilder
  money(nameOrOptions: string | DecimalFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "money", precision: 19, scale: 4 })
    }
    return this.addField({ precision: 19, scale: 4, ...nameOrOptions, type: "money" })
  }

  // ─── tipos booleanos y fechas ─────────────────────────────────────────────

  bool(name: string): FieldBuilder
  bool(options: BoolFieldOptions): FieldBuilder
  bool(nameOrOptions: string | BoolFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "bool" })
    }
    return this.addField({ ...nameOrOptions, type: "bool" })
  }

  date(name: string): FieldBuilder
  date(options: DateFieldOptions): FieldBuilder
  date(nameOrOptions: string | DateFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "date" })
    }
    return this.addField({ ...nameOrOptions, type: "date" })
  }

  datetime(name: string): FieldBuilder
  datetime(options: DateFieldOptions): FieldBuilder
  datetime(nameOrOptions: string | DateFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "datetime" })
    }
    return this.addField({ ...nameOrOptions, type: "datetime" })
  }

  timestamp(name: string): FieldBuilder
  timestamp(options: DateFieldOptions): FieldBuilder
  timestamp(nameOrOptions: string | DateFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "timestamp" })
    }
    return this.addField({ ...nameOrOptions, type: "timestamp" })
  }

  // ─── tipos especiales ─────────────────────────────────────────────────────

  uuid(name: string): FieldBuilder
  uuid(options: SimpleFieldOptions): FieldBuilder
  uuid(nameOrOptions: string | SimpleFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "uuid" })
    }
    return this.addField({ ...nameOrOptions, type: "uuid" })
  }

  objectId(name: string): FieldBuilder
  objectId(options: SimpleFieldOptions): FieldBuilder
  objectId(nameOrOptions: string | SimpleFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "objectId" })
    }
    return this.addField({ ...nameOrOptions, type: "objectId" })
  }

  binary(name: string): FieldBuilder
  binary(options: SimpleFieldOptions): FieldBuilder
  binary(nameOrOptions: string | SimpleFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "binary" })
    }
    return this.addField({ ...nameOrOptions, type: "binary" })
  }

  inet(name: string): FieldBuilder
  inet(options: SimpleFieldOptions): FieldBuilder
  inet(nameOrOptions: string | SimpleFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "inet" })
    }
    return this.addField({ ...nameOrOptions, type: "inet" })
  }

  // ─── tipos complejos ──────────────────────────────────────────────────────

  enum(name: string, values: string[]): FieldBuilder
  enum(options: EnumFieldOptions): FieldBuilder
  enum(nameOrOptions: string | EnumFieldOptions, values?: string[]): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "enum", values })
    }
    return this.addField({ ...nameOrOptions, type: "enum" })
  }

  json(name: string): FieldBuilder
  json(options: JsonFieldOptions): FieldBuilder
  json(nameOrOptions: string | JsonFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "json" })
    }
    return this.addField({ ...nameOrOptions, type: "json" })
  }

  array(name: string): FieldBuilder
  array(options: ArrayFieldOptions): FieldBuilder
  array(nameOrOptions: string | ArrayFieldOptions): FieldBuilder {
    if (typeof nameOrOptions === "string") {
      return this.addField({ name: nameOrOptions, type: "array" })
    }
    return this.addField({ ...nameOrOptions, type: "array" })
  }
}
