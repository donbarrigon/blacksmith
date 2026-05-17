import { existsSync } from "node:fs"
import { readFile, writeFile } from "node:fs/promises"
import { join } from "node:path"

const LOG_PATH = join(process.cwd(), "tmp", "migrations.json")

export interface MigrationLog {
  id: number
  collection: string
  action: "up" | "down"
  status: "success" | "failed"
  executedAt: string
  file: string
  message: string
}

export async function readLog(): Promise<MigrationLog[]> {
  if (!existsSync(LOG_PATH)) {
    return []
  }
  const raw = await readFile(LOG_PATH, "utf-8")
  const trimmed = raw.trim()
  if (!trimmed || trimmed === "[]") {
    return []
  }
  return JSON.parse(trimmed) as MigrationLog[]
}

export async function appendLog(entries: MigrationLog[]): Promise<void> {
  const existing = await readLog()
  const updated = [...existing, ...entries]
  await writeLog(updated)
}

export async function writeLog(entries: MigrationLog[]): Promise<void> {
  const lines = entries.map((e) => JSON.stringify(e))
  const content = "[\n" + lines.join(",\n") + "\n]\n"
  await writeFile(LOG_PATH, content, "utf-8")
}

export async function clearLog(): Promise<void> {
  await writeFile(LOG_PATH, "[]\n", "utf-8")
}

export function getNextBatchId(log: MigrationLog[]): number {
  if (log.length === 0) return 1
  return Math.max(...log.map((e) => e.id)) + 1
}

/**
 * Returns the last action per file+collection.
 * Used to determine what has been run and what hasn't.
 */
export function getLastActionMap(log: MigrationLog[]): Map<string, MigrationLog> {
  const map = new Map<string, MigrationLog>()
  for (const entry of log) {
    const key = `${entry.file}::${entry.collection}`
    map.set(key, entry)
  }
  return map
}

/**
 * Returns all unique batch ids that have a successful "up" action,
 * sorted descending (most recent first).
 */
export function getSuccessfulBatchIds(log: MigrationLog[]): number[] {
  const lastActionMap = getLastActionMap(log)
  const ids = new Set<number>()
  for (const entry of lastActionMap.values()) {
    if (entry.action === "up" && entry.status === "success") {
      ids.add(entry.id)
    }
  }
  return Array.from(ids).sort((a, b) => b - a)
}
