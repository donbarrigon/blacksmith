import type { FieldDefinition, ForeignDefinition, IndexDefinition } from "../reader"

// ================================================================
//              TYPE MAPPING
// ================================================================

export function toPgType(field: FieldDefinition, tableName: string): string {
  const length = field.length ?? 255

  switch (field.type) {
    case "int8":
      return "SMALLINT"
    case "int16":
      return "SMALLINT"
    case "int":
      return field.autoIncrement ? "SERIAL" : "INTEGER"
    case "int64":
      return field.autoIncrement ? "BIGSERIAL" : "BIGINT"
    case "uint8":
      return "SMALLINT"
    case "uint16":
      return "INTEGER"
    case "uint":
      return field.autoIncrement ? "SERIAL" : "INTEGER"
    case "uint64":
      return field.autoIncrement ? "BIGSERIAL" : "BIGINT"
    case "float":
      return "REAL"
    case "double":
      return "DOUBLE PRECISION"
    case "decimal": {
      if (field.precision) {
        return field.scale !== undefined ? `NUMERIC(${field.precision}, ${field.scale})` : `NUMERIC(${field.precision})`
      }
      return "NUMERIC"
    }
    case "string":
      return `VARCHAR(${length})`
    case "text":
      return "TEXT"
    case "char":
      return `CHAR(${length})`
    case "bool":
      return "BOOLEAN"
    case "date":
      return "DATE"
    case "datetime":
      return "TIMESTAMP"
    case "timestamp":
      return "TIMESTAMPTZ"
    case "objectId":
      return "VARCHAR(24)"
    case "uuid":
      return "UUID"
    case "json":
      return "JSONB"
    case "array":
      return "JSONB"
    case "enum":
      return `${tableName}_${field.name}` // tipo creado con CREATE TYPE
    case "binary":
      return "BYTEA"
    case "inet":
      return "INET"
    case "money":
      return "MONEY"
    default:
      return "TEXT"
  }
}

// ================================================================
//              ENUM TYPE
// ================================================================

export function buildCreateEnumType(field: FieldDefinition, tableName: string): string {
  const typeName = `${tableName}_${field.name}`
  const values = (field.values ?? []).map((v) => `'${v}'`).join(", ")
  return `CREATE TYPE ${typeName} AS ENUM (${values})`
}

// ================================================================
//              COLUMN DEFINITION
// ================================================================

export function buildColumnDef(field: FieldDefinition, tableName: string): string {
  const parts: string[] = []

  parts.push(`"${field.name}"`)
  parts.push(toPgType(field, tableName))

  if (field.primary) {
    parts.push("PRIMARY KEY")
  }

  if (!field.nullable && !field.primary) {
    parts.push("NOT NULL")
  }

  if (field.default !== undefined && field.default !== null) {
    const def = typeof field.default === "string" ? `'${field.default}'` : String(field.default)
    parts.push(`DEFAULT ${def}`)
  }

  return parts.join(" ")
}

// ================================================================
//              CREATE TABLE
// ================================================================

export function buildCreateTable(tableName: string, fields: FieldDefinition[]): string {
  const columns = fields.map((f) => `  ${buildColumnDef(f, tableName)}`).join(",\n")
  return `CREATE TABLE IF NOT EXISTS "${tableName}" (\n${columns}\n)`
}

// ================================================================
//              ALTER TABLE ADD COLUMN
// ================================================================

export function buildAddColumns(tableName: string, fields: FieldDefinition[]): string[] {
  return fields.map((f) => `ALTER TABLE "${tableName}" ADD COLUMN IF NOT EXISTS ${buildColumnDef(f, tableName)}`)
}

// ================================================================
//              ALTER TABLE ALTER COLUMN
// ================================================================

export function buildAlterColumns(tableName: string, fields: FieldDefinition[]): string[] {
  const statements: string[] = []
  for (const f of fields) {
    const pgType = toPgType(f, tableName)
    statements.push(`ALTER TABLE "${tableName}" ALTER COLUMN "${f.name}" TYPE ${pgType} USING "${f.name}"::${pgType}`)
    if (!f.nullable) {
      statements.push(`ALTER TABLE "${tableName}" ALTER COLUMN "${f.name}" SET NOT NULL`)
    } else {
      statements.push(`ALTER TABLE "${tableName}" ALTER COLUMN "${f.name}" DROP NOT NULL`)
    }
    if (f.default !== undefined && f.default !== null) {
      const def = typeof f.default === "string" ? `'${f.default}'` : String(f.default)
      statements.push(`ALTER TABLE "${tableName}" ALTER COLUMN "${f.name}" SET DEFAULT ${def}`)
    } else {
      statements.push(`ALTER TABLE "${tableName}" ALTER COLUMN "${f.name}" DROP DEFAULT`)
    }
  }
  return statements
}

// ================================================================
//              DROP COLUMNS
// ================================================================

export function buildDropColumns(tableName: string, fieldNames: string[]): string[] {
  return fieldNames.map((name) => `ALTER TABLE "${tableName}" DROP COLUMN IF EXISTS "${name}"`)
}

// ================================================================
//              DROP TABLE
// ================================================================

export function buildDropTable(tableName: string): string {
  return `DROP TABLE IF EXISTS "${tableName}" CASCADE`
}

// ================================================================
//              INDEXES
// ================================================================

const ORDER_MAP: Record<string, string> = { asc: "ASC", desc: "DESC" }

function resolveForeign(
  field: string | string[],
  foreign: ForeignDefinition,
): { collection: string; field: string; onUpdate: string; onDelete: string } {
  const onUpdate = foreign.onUpdate ?? "cascade"
  const onDelete = foreign.onDelete ?? "cascade"

  if (foreign.auto) {
    const fieldName = Array.isArray(field) ? field[0] : field
    const collection = fieldName.replace(/_id$/, "") + "s" // simple pluralization
    return { collection, field: "id", onUpdate, onDelete }
  }

  return {
    collection: foreign.collection!,
    field: foreign.field!,
    onUpdate,
    onDelete,
  }
}

function toSqlAction(action: string): string {
  return action.replace("_", " ").toUpperCase()
}

export function buildCreateIndex(tableName: string, def: IndexDefinition): string[] {
  const statements: string[] = []
  const fields = Array.isArray(def.field) ? def.field : [def.field]
  const order = ORDER_MAP[def.order ?? "asc"]
  const indexName = def.name ?? fields.map((f) => `${tableName}_${f}`).join("_") + "_idx"
  const unique = def.unique ? "UNIQUE " : ""
  const cols = fields.map((f) => `"${f}" ${order}`).join(", ")

  statements.push(`CREATE ${unique}INDEX IF NOT EXISTS "${indexName}" ON "${tableName}" (${cols})`)

  // foreign key
  if (def.foreign) {
    const resolved = resolveForeign(def.field, def.foreign)
    const fkName = `${tableName}_${fields[0]}_fk`
    statements.push(
      `ALTER TABLE "${tableName}" ADD CONSTRAINT "${fkName}" ` +
        `FOREIGN KEY ("${fields[0]}") REFERENCES "${resolved.collection}" ("${resolved.field}") ` +
        `ON UPDATE ${toSqlAction(resolved.onUpdate)} ON DELETE ${toSqlAction(resolved.onDelete)}`,
    )
  }

  return statements
}

export function buildDropIndex(indexName: string): string {
  return `DROP INDEX IF EXISTS "${indexName}"`
}

export function buildDropForeignKey(tableName: string, fieldName: string): string {
  return `ALTER TABLE "${tableName}" DROP CONSTRAINT IF EXISTS "${tableName}_${fieldName}_fk"`
}

export function buildAlterIndex(tableName: string, def: IndexDefinition): string[] {
  const fields = Array.isArray(def.field) ? def.field : [def.field]
  const indexName = def.name ?? fields.map((f) => `${tableName}_${f}`).join("_") + "_idx"
  return [buildDropIndex(indexName), ...buildCreateIndex(tableName, def)]
}
