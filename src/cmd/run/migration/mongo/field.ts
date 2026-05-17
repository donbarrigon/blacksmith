import type { FieldDefinition } from "../reader"

// ================================================================
//              FIELD TYPE → BSON TYPE MAPPING
// ================================================================

const TYPE_MAP: Record<string, string> = {
  int8: "int",
  int16: "int",
  int: "int",
  int64: "long",
  uint8: "int",
  uint16: "int",
  uint: "long",
  uint64: "decimal",
  float: "double",
  double: "double",
  decimal: "decimal",
  string: "string",
  text: "string",
  char: "string",
  bool: "bool",
  date: "date",
  datetime: "date",
  timestamp: "timestamp",
  objectId: "objectId",
  uuid: "binData",
  json: "object",
  array: "array",
  enum: "string",
  binary: "binData",
  inet: "string",
  money: "decimal",
}

function toBsonType(field: FieldDefinition): string | string[] {
  const bson = TYPE_MAP[field.type] ?? "string"
  if (field.nullable) {
    return [bson, "null"]
  }
  return bson
}

// ================================================================
//              FIELD → BSON PROPERTY
// ================================================================

function buildBsonProperty(field: FieldDefinition): Record<string, unknown> {
  const prop: Record<string, unknown> = {
    bsonType: toBsonType(field),
  }

  if (field.description) {
    prop.description = field.description
  }

  // string constraints
  if (field.type === "string" || field.type === "char" || field.type === "inet") {
    if (field.length) prop.maxLength = field.length
    if (field.type === "char" && field.length) prop.minLength = field.length
  }

  // text has no length limit
  if (field.type === "text") {
    prop.minLength = 0
  }

  // enum values
  if (field.type === "enum" && field.values) {
    prop.enum = field.values
  }

  // decimal precision and scale
  if ((field.type === "decimal" || field.type === "money") && field.precision) {
    prop.description = [
      field.description,
      `precision: ${field.precision}`,
      field.scale !== undefined ? `scale: ${field.scale}` : "",
    ]
      .filter(Boolean)
      .join(" | ")
  }

  // nested json object
  if (field.type === "json" && field.jsonSchema && field.jsonSchema.length > 0) {
    const nested = buildJsonSchema(field.jsonSchema, true)
    prop.bsonType = field.nullable ? ["object", "null"] : "object"
    prop.required = nested.required
    prop.additionalProperties = nested.additionalProperties
    prop.properties = nested.properties
  }

  // array items
  if (field.type === "array" && field.items) {
    prop.bsonType = field.nullable ? ["array", "null"] : "array"
    prop.items = { bsonType: toBsonType(field.items) }
  }

  return prop
}

// ================================================================
//              FIELDS → $jsonSchema
// ================================================================

export interface JsonSchema {
  bsonType: "object"
  required: string[]
  additionalProperties: boolean
  properties: Record<string, unknown>
}

export function buildJsonSchema(fields: FieldDefinition[], additionalProperties: boolean = false): JsonSchema {
  const required: string[] = []
  const properties: Record<string, unknown> = {}

  for (const field of fields) {
    // _id is always managed by mongo, skip required
    if (field.name === "_id") {
      properties[field.name] = { bsonType: "objectId" }
      continue
    }

    properties[field.name] = buildBsonProperty(field)

    if (!field.nullable) {
      required.push(field.name)
    }
  }

  return {
    bsonType: "object",
    required,
    additionalProperties,
    properties,
  }
}

// ================================================================
//              MERGE SCHEMAS (for alterField)
// ================================================================

export function mergeJsonSchema(
  existing: JsonSchema,
  newFields: FieldDefinition[],
  additionalProperties: boolean,
): JsonSchema {
  const patch = buildJsonSchema(newFields, additionalProperties)

  const required = Array.from(new Set([...existing.required, ...patch.required]))
  const properties = { ...existing.properties, ...patch.properties }

  return {
    bsonType: "object",
    required,
    additionalProperties,
    properties,
  }
}

// ================================================================
//              DROP FIELDS FROM SCHEMA
// ================================================================

export function dropFromJsonSchema(existing: JsonSchema, fieldNames: string[]): JsonSchema {
  const properties = { ...existing.properties }
  const required = existing.required.filter((r) => !fieldNames.includes(r))

  for (const name of fieldNames) {
    delete properties[name]
  }

  return {
    ...existing,
    required,
    properties,
  }
}
