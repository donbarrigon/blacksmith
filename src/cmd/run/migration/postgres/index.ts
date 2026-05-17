import { printWarning } from "../../../../utils/print"
import type { CollectionOperations, FieldDefinition } from "../reader"
import {
  buildAddColumns,
  buildAlterColumns,
  buildAlterIndex,
  buildCreateEnumType,
  buildCreateIndex,
  buildCreateTable,
  buildDropColumns,
  buildDropForeignKey,
  buildDropIndex,
  buildDropTable,
} from "./builder"

// ================================================================
//              CONNECTION
// ================================================================

export function createPgClient(connectionString: string) {
  return new Bun.SQL(connectionString)
}

// ================================================================
//              ENUM TYPES
// ================================================================

async function ensureEnumTypes(
  sql: InstanceType<typeof Bun.SQL>,
  tableName: string,
  fields: FieldDefinition[],
): Promise<void> {
  const enumFields = fields.filter((f) => f.type === "enum")
  for (const field of enumFields) {
    const typeName = `${tableName}_${field.name}`
    const existing = await sql`
      SELECT 1 FROM pg_type WHERE typname = ${typeName}
    `
    if (existing.length > 0) {
      printWarning(`Enum type "${typeName}" already exists — skipping`)
      continue
    }
    const ddl = buildCreateEnumType(field, tableName)
    await sql.unsafe(ddl)
  }
}

async function dropEnumTypes(
  sql: InstanceType<typeof Bun.SQL>,
  tableName: string,
  fields: FieldDefinition[],
): Promise<void> {
  const enumFields = fields.filter((f) => f.type === "enum")
  for (const field of enumFields) {
    const typeName = `${tableName}_${field.name}`
    await sql.unsafe(`DROP TYPE IF EXISTS "${typeName}"`)
  }
}

// ================================================================
//              TABLE EXISTS
// ================================================================

async function tableExists(sql: InstanceType<typeof Bun.SQL>, tableName: string): Promise<boolean> {
  const result = await sql`
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = ${tableName}
  `
  return result.length > 0
}

// ================================================================
//              RUN STATEMENTS IN TRANSACTION
// ================================================================

async function runInTransaction(sql: InstanceType<typeof Bun.SQL>, statements: string[]): Promise<void> {
  await sql.unsafe("BEGIN")
  try {
    for (const stmt of statements) {
      await sql.unsafe(stmt)
    }
    await sql.unsafe("COMMIT")
  } catch (err) {
    await sql.unsafe("ROLLBACK")
    throw err
  }
}

// ================================================================
//              UP
// ================================================================

export async function runPostgresUp(
  sql: InstanceType<typeof Bun.SQL>,
  tableName: string,
  ops: CollectionOperations,
): Promise<void> {
  const exists = await tableExists(sql, tableName)
  const statements: string[] = []

  if (!exists) {
    // ── crear tabla ──────────────────────────────────────────────
    if (ops.createField && ops.createField.length > 0) {
      await ensureEnumTypes(sql, tableName, ops.createField)
      statements.push(buildCreateTable(tableName, ops.createField))
    }
  } else {
    // ── agregar columnas ─────────────────────────────────────────
    if (ops.createField && ops.createField.length > 0) {
      await ensureEnumTypes(sql, tableName, ops.createField)
      statements.push(...buildAddColumns(tableName, ops.createField))
    }

    // ── alterar columnas ─────────────────────────────────────────
    if (ops.alterField && ops.alterField.length > 0) {
      await ensureEnumTypes(sql, tableName, ops.alterField)
      statements.push(...buildAlterColumns(tableName, ops.alterField))
    }
  }

  // ── eliminar columnas ────────────────────────────────────────
  if (ops.dropField && ops.dropField.length > 0) {
    statements.push(...buildDropColumns(tableName, ops.dropField))
  }

  // ── indices ──────────────────────────────────────────────────
  if (ops.createIndex && ops.createIndex.length > 0) {
    for (const def of ops.createIndex) {
      statements.push(...buildCreateIndex(tableName, def))
    }
  }

  if (ops.alterIndex && ops.alterIndex.length > 0) {
    for (const def of ops.alterIndex) {
      statements.push(...buildAlterIndex(tableName, def))
    }
  }

  if (ops.dropIndex && ops.dropIndex.length > 0) {
    for (const name of ops.dropIndex) {
      statements.push(buildDropIndex(name))
    }
  }

  if (statements.length > 0) {
    await runInTransaction(sql, statements)
  }
}

// ================================================================
//              DOWN
// ================================================================

export async function runPostgresDown(
  sql: InstanceType<typeof Bun.SQL>,
  tableName: string,
  ops: CollectionOperations,
): Promise<void> {
  const statements: string[] = []

  // ── drop tabla ───────────────────────────────────────────────
  if (ops.dropCollection) {
    if (ops.createField) {
      await dropEnumTypes(sql, tableName, ops.createField)
    }
    statements.push(buildDropTable(tableName))
    await runInTransaction(sql, statements)
    return
  }

  // ── drop indices y foreign keys ──────────────────────────────
  if (ops.dropIndex && ops.dropIndex.length > 0) {
    for (const name of ops.dropIndex) {
      statements.push(buildDropIndex(name))
    }
  }

  // ── alter index ──────────────────────────────────────────────
  if (ops.alterIndex && ops.alterIndex.length > 0) {
    for (const def of ops.alterIndex) {
      statements.push(...buildAlterIndex(tableName, def))
    }
  }

  // ── crear indices (restaurar los eliminados en up) ───────────
  if (ops.createIndex && ops.createIndex.length > 0) {
    for (const def of ops.createIndex) {
      // si tenia foreign key eliminarla antes de recrear
      if (def.foreign) {
        const field = Array.isArray(def.field) ? (def.field[0] as string) : def.field
        statements.push(buildDropForeignKey(tableName, field))
      }
      statements.push(...buildCreateIndex(tableName, def))
    }
  }

  // ── drop columnas ────────────────────────────────────────────
  if (ops.dropField && ops.dropField.length > 0) {
    statements.push(...buildDropColumns(tableName, ops.dropField))
  }

  // ── alter columnas (restaurar) ───────────────────────────────
  if (ops.alterField && ops.alterField.length > 0) {
    statements.push(...buildAlterColumns(tableName, ops.alterField))
  }

  // ── crear columnas (restaurar las eliminadas en up) ──────────
  if (ops.createField && ops.createField.length > 0) {
    await ensureEnumTypes(sql, tableName, ops.createField)
    statements.push(...buildAddColumns(tableName, ops.createField))
  }

  if (statements.length > 0) {
    await runInTransaction(sql, statements)
  }
}
