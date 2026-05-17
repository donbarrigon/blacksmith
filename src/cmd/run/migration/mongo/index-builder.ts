import type { CreateIndexesOptions, IndexSpecification } from "mongodb"
import type { IndexDefinition, IndexModel } from "../reader"

// ================================================================
//              INDEX TYPE MAPPING
// ================================================================

function toIndexValue(indexType: string, order: "asc" | "desc" = "asc"): 1 | -1 | string {
  const specialTypes = ["text", "hashed", "2dsphere", "2d", "wildcard"]
  if (specialTypes.includes(indexType)) {
    return indexType
  }
  return order === "asc" ? 1 : -1
}

// ================================================================
//              BUILD INDEX KEY
// ================================================================

function buildIndexKey(def: IndexDefinition): IndexSpecification {
  const indexType = def.type ?? "index"
  const order = def.order ?? "asc"
  const key: Record<string, 1 | -1 | string> = {}

  const fields = Array.isArray(def.field) ? def.field : [def.field]
  for (const field of fields) {
    key[field] = toIndexValue(indexType, order)
  }

  return key as IndexSpecification
}

// ================================================================
//              BUILD INDEX OPTIONS
// ================================================================

function buildIndexOptions(def: IndexDefinition): CreateIndexesOptions {
  const options: CreateIndexesOptions = {}

  if (def.name) {
    options.name = def.name
  } else {
    const fields = Array.isArray(def.field) ? def.field : [def.field]
    const order = def.order === "desc" ? -1 : 1
    options.name = fields.map((f) => `${f}_${order}`).join("_")
  }

  if (def.unique) options.unique = true
  if (def.sparse) options.sparse = true
  if (def.background) options.background = true
  if (def.expireAfterSeconds !== undefined) options.expireAfterSeconds = def.expireAfterSeconds
  if (def.collation) options.collation = def.collation as CreateIndexesOptions["collation"]

  return options
}

// ================================================================
//              EXPORTED INDEX MODELS
// ================================================================

export interface MongoIndexModel {
  key: IndexSpecification
  options: CreateIndexesOptions
}

export function buildCreateIndexModels(defs: IndexDefinition[]): MongoIndexModel[] {
  return defs.map((def) => ({
    key: buildIndexKey(def),
    options: buildIndexOptions(def),
  }))
}

export function buildRawIndexModels(models: IndexModel[]): MongoIndexModel[] {
  return models.map((m) => ({
    key: m.model as IndexSpecification,
    options: (m.options ?? {}) as CreateIndexesOptions,
  }))
}

export function extractIndexNames(defs: IndexDefinition[]): string[] {
  return defs.map((def) => {
    if (def.name) return def.name
    const fields = Array.isArray(def.field) ? def.field : [def.field]
    const order = def.order === "desc" ? -1 : 1
    return fields.map((f) => `${f}_${order}`).join("_")
  })
}
