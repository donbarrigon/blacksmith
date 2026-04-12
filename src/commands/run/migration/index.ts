import { existsSync, mkdirSync } from "node:fs"
import { join } from "node:path"
import { confirm } from "@inquirer/prompts"
import { MongoClient } from "mongodb"
import { printError, printInfo, printSuccess, printWarning } from "../../../cmd/print"
import type { Args } from "../../../types/Args"
import {
  appendLog,
  clearLog,
  getLastActionMap,
  getNextBatchId,
  getSuccessfulBatchIds,
  type MigrationLog,
  readLog,
} from "./log"
import { runMongoDown, runMongoUp } from "./mongo"
import { getMigrationFiles, parseMigrationFile } from "./reader"

// ================================================================
//              ENV
// ================================================================

interface EnvDb {
  driver: "mongodb" | "postgresql" | "mariadb" | "mysql"
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
//              ENSURE TMP DIR
// ================================================================

function ensureTmpDir(): void {
  const tmpDir = join(process.cwd(), "tmp")
  if (!existsSync(tmpDir)) {
    mkdirSync(tmpDir, { recursive: true })
  }
}

// ================================================================
//              RUN UP
// ================================================================

async function runUp(env: EnvDb): Promise<void> {
  const allFiles = await getMigrationFiles()
  const log = await readLog()
  const lastActionMap = getLastActionMap(log)

  // archivos pendientes: los que no tienen un up exitoso como ultima accion
  const pending = allFiles.filter((file) => {
    const entries = [...lastActionMap.entries()].filter(([key]) => key.startsWith(`${file}::`)).map(([, v]) => v)
    if (entries.length === 0) return true
    return entries.some((e) => !(e.action === "up" && e.status === "success"))
  })

  if (pending.length === 0) {
    printInfo("Nothing to migrate. All files are up to date.")
    return
  }

  const batchId = getNextBatchId(log)
  printInfo(`Running ${pending.length} migration(s) — batch #${batchId}`)

  if (env.driver === "mongodb") {
    await runMongoMigrations(env, pending, batchId, "up")
  } else {
    console.log(`TODO: pendiente por hacer — driver: ${env.driver}`)
  }
}

// ================================================================
//              RUN ROLLBACK
// ================================================================

async function runRollback(env: EnvDb, steps: number): Promise<void> {
  const log = await readLog()
  const batchIds = getSuccessfulBatchIds(log)

  if (batchIds.length === 0) {
    printInfo("Nothing to rollback.")
    return
  }

  const toRollback = batchIds.slice(0, steps)
  printInfo(`Rolling back ${toRollback.length} batch(es): #${toRollback.join(", #")}`)

  // archivos de esos batches en orden inverso
  const entries = log.filter((e) => toRollback.includes(e.id) && e.action === "up" && e.status === "success").reverse()

  const files = [...new Set(entries.map((e) => e.file))].reverse()
  const batchId = getNextBatchId(log)

  if (env.driver === "mongodb") {
    await runMongoMigrations(env, files, batchId, "down")
  } else {
    console.log(`TODO: pendiente por hacer — driver: ${env.driver}`)
  }
}

// ================================================================
//              RUN RESET
// ================================================================

async function runReset(env: EnvDb): Promise<void> {
  const log = await readLog()
  const batchIds = getSuccessfulBatchIds(log)

  if (batchIds.length === 0) {
    printInfo("Nothing to reset.")
    return
  }

  const entries = log.filter((e) => e.action === "up" && e.status === "success").reverse()

  const files = [...new Set(entries.map((e) => e.file))].reverse()
  const batchId = getNextBatchId(log)

  printInfo(`Resetting all migrations — ${files.length} file(s)`)

  if (env.driver === "mongodb") {
    await runMongoMigrations(env, files, batchId, "down")
  } else {
    console.log(`TODO: pendiente por hacer — driver: ${env.driver}`)
  }
}

// ================================================================
//              RUN REFRESH
// ================================================================

async function runRefresh(env: EnvDb): Promise<void> {
  printInfo("Refreshing all migrations...")
  await runReset(env)
  await runUp(env)
}

// ================================================================
//              RUN FRESH
// ================================================================

async function runFresh(env: EnvDb): Promise<void> {
  printWarning("This will drop all collections/tables and clear the migration history.")

  if (env.driver === "mongodb") {
    const client = new MongoClient(env.string)
    try {
      await client.connect()
      const db = client.db(env.name)
      const collections = await db.listCollections().toArray()
      for (const col of collections) {
        await db.collection(col.name).drop()
        printSuccess(`Dropped collection: ${col.name}`)
      }
    } finally {
      await client.close()
    }
  } else {
    console.log(`TODO: pendiente por hacer — driver: ${env.driver}`)
  }

  await clearLog()
  printSuccess("Migration history cleared.")
  printInfo("Run 'fg run migration up' to re-run all migrations.")
}

// ================================================================
//              MONGO MIGRATION RUNNER
// ================================================================

async function runMongoMigrations(env: EnvDb, files: string[], batchId: number, action: "up" | "down"): Promise<void> {
  const client = new MongoClient(env.string)

  try {
    await client.connect()

    // si la bd no existe preguntar si se crea
    // const adminDb = client.db("admin")
    // const dbList = await adminDb.admin().listDatabases()
    // const dbExists = dbList.databases.some((d: { name: string }) => d.name === env.name)

    // if (!dbExists && action === "up") {
    //   const create = await confirm({
    //     message: `Database "${env.name}" does not exist. Create it?`,
    //     default: true,
    //   })
    //   if (!create) {
    //     printWarning("Migration cancelled.")
    //     return
    //   }
    // }

    const db = client.db(env.name)
    const newEntries: MigrationLog[] = []

    for (const file of files) {
      const parsed = await parseMigrationFile(file)
      const block = action === "up" ? parsed.content.up : parsed.content.down
      const collections = Object.keys(block)

      for (const collectionName of collections) {
        const ops = block[collectionName]
        const executedAt = new Date().toISOString()

        try {
          if (!ops) {
            printError(`Migration file "${file}" has an Invalid migration ops: ${ops}`)
            printError("\nMigration cancelled.")
            return
          }
          if (action === "up") {
            await runMongoUp(db, collectionName, ops)
          } else {
            await runMongoDown(db, collectionName, ops)
          }

          printSuccess(`${action.toUpperCase()} ${file} → ${collectionName}`)
          newEntries.push({
            id: batchId,
            collection: collectionName,
            action,
            status: "success",
            executedAt,
            file,
            message: "",
          })
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err)
          printError(`FAILED ${file} → ${collectionName}: ${message}`)
          newEntries.push({
            id: batchId,
            collection: collectionName,
            action,
            status: "failed",
            executedAt,
            file,
            message,
          })

          // guardar lo que se ejecuto hasta el error y detener
          await appendLog(newEntries)
          printWarning("Migration stopped. Fix the error and run again.")
          return
        }
      }
    }

    await appendLog(newEntries)
    printSuccess(`Done. Batch #${batchId} — ${newEntries.length} collection(s) migrated.`)
  } finally {
    await client.close()
  }
}

// ================================================================
//              ENTRY POINT
// ================================================================

export async function runMigration(args: Args): Promise<void> {
  const action = args.flags[1] ?? "up"
  const steps = args.flags[2] ? Number(args.flags[2]) : 1

  ensureTmpDir()

  const env = await loadEnv()

  switch (action) {
    case "up":
      await runUp(env.db)
      break

    case "rollback":
      await runRollback(env.db, steps)
      break

    case "reset":
      await runReset(env.db)
      break

    case "refresh":
      await runRefresh(env.db)
      break

    case "fresh":
      await runFresh(env.db)
      break

    default:
      throw new Error(`Unknown migration action: ${action}`)
  }
}
