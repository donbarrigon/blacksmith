import { existsSync } from "node:fs"
import { join } from "node:path"
import { MongoClient } from "mongodb"

// ================================================================
//              ENV
// ================================================================

interface EnvDb {
  driver: string
  name: string
  string: string
}

interface Env {
  db: EnvDb
}

async function loadEnv(): Promise<Env> {
  const envPath = join(process.cwd(), "env.json")
  if (!existsSync(envPath)) {
    throw new Error("env.json not found in project root")
  }
  const file = Bun.file(envPath)
  return (await file.json()) as Env
}

// ================================================================
//              COLORS
// ================================================================

const c = {
  reset: "\x1b[0m",
  primary: "\x1b[95m",
  secondary: "\x1b[94m",
  info: "\x1b[96m",
  warning: "\x1b[93m",
  success: "\x1b[92m",
  danger: "\x1b[91m",
  gray: "\x1b[90m",
  bold: "\x1b[1m",
  dim: "\x1b[2m",
}

const hr = `${c.gray}${"─".repeat(60)}${c.reset}`

function label(text: string): string {
  return `${c.bold}${c.primary}${text}${c.reset}`
}

function key(text: string): string {
  return `${c.info}${text}${c.reset}`
}

function val(text: unknown): string {
  return `${c.warning}${JSON.stringify(text, null, 2)}${c.reset}`
}

// ================================================================
//              PRINT HELPERS
// ================================================================

function printFields(properties: Record<string, unknown>, required: string[], indent = 0) {
  const pad = " ".repeat(indent)
  for (const [fieldName, def] of Object.entries(properties)) {
    const d = def as Record<string, unknown>
    const isRequired = required.includes(fieldName)
    const bsonType = Array.isArray(d.bsonType) ? d.bsonType.join(" | ") : d.bsonType
    const nullable = Array.isArray(d.bsonType) && d.bsonType.includes("null")

    const tags: string[] = []
    if (isRequired) tags.push(`${c.success}required${c.reset}`)
    if (nullable) tags.push(`${c.gray}nullable${c.reset}`)
    if (d.enum) tags.push(`${c.secondary}enum${c.reset}`)
    if (d.minLength !== undefined) tags.push(`${c.dim}minLength:${d.minLength}${c.reset}`)
    if (d.maxLength !== undefined) tags.push(`${c.dim}maxLength:${d.maxLength}${c.reset}`)

    const tagStr = tags.length > 0 ? `  ${tags.join("  ")}` : ""
    console.log(`${pad}  ${key(fieldName)}${c.gray}:${c.reset} ${c.warning}${bsonType}${c.reset}${tagStr}`)

    if (d.enum) {
      console.log(`${pad}    ${c.dim}values: ${JSON.stringify(d.enum)}${c.reset}`)
    }

    if (d.description) {
      console.log(`${pad}    ${c.dim}// ${d.description}${c.reset}`)
    }

    // nested object
    if (d.properties && typeof d.properties === "object") {
      printFields(d.properties as Record<string, unknown>, (d.required as string[]) ?? [], indent + 4)
    }

    // array items
    if (d.items && typeof d.items === "object") {
      const items = d.items as Record<string, unknown>
      console.log(`${pad}    ${c.dim}items: ${JSON.stringify(items.bsonType)}${c.reset}`)
    }
  }
}

function printIndexes(indexes: Record<string, unknown>[]) {
  for (const idx of indexes) {
    const name = idx.name as string
    const key2 = idx.key as Record<string, unknown>
    const unique = idx.unique ? `  ${c.success}unique${c.reset}` : ""
    const sparse = idx.sparse ? `  ${c.warning}sparse${c.reset}` : ""
    const ttl = idx.expireAfterSeconds !== undefined ? `  ${c.info}TTL:${idx.expireAfterSeconds}s${c.reset}` : ""

    console.log(`  ${key(name)}${unique}${sparse}${ttl}`)
    console.log(`  ${c.dim}  key: ${JSON.stringify(key2)}${c.reset}`)
  }
}

// ================================================================
//              MAIN
// ================================================================

async function main() {
  const collectionName = process.argv[2]
  if (!collectionName) {
    console.error(`${c.danger}Usage: bun test-mongo.ts <collectionName>${c.reset}`)
    process.exit(1)
  }

  const env = await loadEnv()

  if (env.db.driver !== "mongodb") {
    console.error(`${c.danger}This script only supports MongoDB${c.reset}`)
    process.exit(1)
  }

  const client = new MongoClient(env.db.string)

  try {
    await client.connect()
    const db = client.db(env.db.name)

    // verificar que la coleccion existe
    const exists = await db.listCollections({ name: collectionName }).toArray()
    if (exists.length === 0) {
      console.error(`${c.danger}Collection "${collectionName}" not found in "${env.db.name}"${c.reset}`)
      process.exit(1)
    }

    // info completa de la coleccion
    const [info] = await db.listCollections({ name: collectionName }, { nameOnly: false }).toArray()

    const collection = db.collection(collectionName)

    // ── NOMBRE ────────────────────────────────────────────────────
    console.log()
    console.log(`${c.bold}${c.primary}  COLLECTION${c.reset}  ${c.warning}${collectionName}${c.reset}`)
    console.log(`  ${c.dim}database: ${env.db.name}${c.reset}`)
    console.log()

    // ── CAMPOS ────────────────────────────────────────────────────
    console.log(label("  FIELDS"))
    console.log(hr)

    const schema = info?.options?.validator?.$jsonSchema as Record<string, unknown> | undefined

    if (schema && schema.properties) {
      printFields(schema.properties as Record<string, unknown>, (schema.required as string[]) ?? [])
    } else {
      console.log(`  ${c.dim}No validator schema defined${c.reset}`)
    }

    console.log()

    // ── INDICES ───────────────────────────────────────────────────
    console.log(label("  INDEXES"))
    console.log(hr)

    const indexes = await collection.indexes()
    printIndexes(indexes as Record<string, unknown>[])
    console.log()

    // ── VALIDATOR ─────────────────────────────────────────────────
    console.log(label("  VALIDATOR"))
    console.log(hr)

    if (info?.options?.validator) {
      const validationLevel = info?.options?.validationLevel ?? "—"
      const validationAction = info?.options?.validationAction ?? "—"
      console.log(`  ${key("validationLevel")}  ${c.warning}${validationLevel}${c.reset}`)
      console.log(`  ${key("validationAction")} ${c.warning}${validationAction}${c.reset}`)
      console.log()
      console.log(val(info.options.validator))
    } else {
      console.log(`  ${c.dim}No validator defined${c.reset}`)
    }

    console.log()

    // ── DATOS ─────────────────────────────────────────────────────
    console.log(label("  DATA"))
    console.log(hr)

    const count = await collection.countDocuments()
    console.log(`  ${c.dim}${count} document(s)${c.reset}`)
    console.log()

    if (count > 0) {
      const docs = await collection.find({}).toArray()
      for (const doc of docs) {
        console.log(val(doc))
        console.log()
      }
    } else {
      console.log(`  ${c.dim}No documents found${c.reset}`)
    }

    console.log()
  } finally {
    await client.close()
  }
}

main().catch((err) => {
  console.error(`${c.danger}Error: ${err.message}${c.reset}`)
  process.exit(1)
})
