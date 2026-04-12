import { existsSync } from "node:fs"
import { readdir, readFile } from "node:fs/promises"
import { join } from "node:path"

const MIGRATION_DIR = join(process.cwd(), "database", "migration")

export interface FieldDefinition {
  name: string
  type: string
  length?: number
  nullable?: boolean
  default?: string | number | boolean | null
  primary?: boolean
  autoIncrement?: boolean
  unsigned?: boolean
  precision?: number
  scale?: number
  values?: string[]
  description?: string
  jsonSchema?: FieldDefinition[]
  items?: FieldDefinition
}

export interface IndexDefinition {
  field: string | string[]
  type?: string
  name?: string
  unique?: boolean
  sparse?: boolean
  nullable?: boolean
  order?: "asc" | "desc"
  background?: boolean
  expireAfterSeconds?: number
  collation?: object
  description?: string
}

export interface IndexModel {
  model: Record<string, 1 | -1 | string>
  options?: Record<string, unknown>
}

export interface CollectionOperations {
  validationLevel?: "strict" | "moderate" | "off"
  validationAction?: "error" | "warn"
  additionalProperties?: boolean
  createField?: FieldDefinition[]
  alterField?: FieldDefinition[]
  dropField?: string[]
  createIndex?: IndexDefinition[]
  alterIndex?: IndexDefinition[]
  dropIndex?: string[]
  indexModel?: IndexModel[]
  dropCollection?: boolean
}

export interface MigrationFile {
  up: Record<string, CollectionOperations>
  down: Record<string, CollectionOperations>
}

export interface ParsedMigration {
  file: string
  content: MigrationFile
}

export async function getMigrationFiles(): Promise<string[]> {
  if (!existsSync(MIGRATION_DIR)) {
    throw new Error(`Migration directory not found: ${MIGRATION_DIR}`)
  }

  const entries = await readdir(MIGRATION_DIR)
  return entries.filter((f) => f.endsWith(".json")).sort() // orden alfabetico garantiza el orden por nombre de archivo
}

export async function parseMigrationFile(file: string): Promise<ParsedMigration> {
  const path = join(MIGRATION_DIR, file)
  const raw = await readFile(path, "utf-8")
  const content = JSON.parse(raw) as MigrationFile
  return { file, content }
}

export async function getPendingFiles(files: string[], executedFiles: Set<string>): Promise<string[]> {
  return files.filter((f) => !executedFiles.has(f))
}
