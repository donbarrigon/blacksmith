import type { Db } from "mongodb"
import type { CollectionOperations } from "../reader"
import { buildJsonSchema, dropFromJsonSchema, mergeJsonSchema } from "./field"
import { buildCreateIndexModels, buildRawIndexModels, extractIndexNames } from "./index-builder"

// ================================================================
//              UP
// ================================================================

export async function runMongoUp(db: Db, collectionName: string, ops: CollectionOperations): Promise<void> {
  const validationLevel = ops.validationLevel ?? "off"
  const validationAction = ops.validationAction ?? "error"
  const additionalProperties = ops.additionalProperties ?? false

  const collectionNames = await db.listCollections({ name: collectionName }).toArray()
  const collectionExists = collectionNames.length > 0

  // ── crear coleccion con validator ──────────────────────────────
  if (!collectionExists) {
    if (validationLevel !== "off" && ops.createField && ops.createField.length > 0) {
      const schema = buildJsonSchema(ops.createField, additionalProperties)
      await db.createCollection(collectionName, {
        validator: { $jsonSchema: schema },
        validationLevel,
        validationAction,
      })
    } else {
      // sin validator, mongo la crea al primer insert
      // pero la creamos explicitamente para poder agregar indices
      await db.createCollection(collectionName)
    }
  } else {
    // ── coleccion existe: alterField ───────────────────────────────
    if (validationLevel !== "off" && ops.alterField && ops.alterField.length > 0) {
      const info = await db.listCollections({ name: collectionName }, { nameOnly: false }).toArray()
      const existingSchema = info[0]?.options?.validator?.$jsonSchema

      if (existingSchema) {
        const merged = mergeJsonSchema(existingSchema, ops.alterField, additionalProperties)
        await db.command({
          collMod: collectionName,
          validator: { $jsonSchema: merged },
          validationLevel,
          validationAction,
        })
      } else {
        const schema = buildJsonSchema(ops.alterField, additionalProperties)
        await db.command({
          collMod: collectionName,
          validator: { $jsonSchema: schema },
          validationLevel,
          validationAction,
        })
      }
    }

    // ── dropField ──────────────────────────────────────────────────
    if (ops.dropField && ops.dropField.length > 0) {
      const info = await db.listCollections({ name: collectionName }, { nameOnly: false }).toArray()
      const existingSchema = info[0]?.options?.validator?.$jsonSchema

      if (existingSchema) {
        const updated = dropFromJsonSchema(existingSchema, ops.dropField)
        await db.command({
          collMod: collectionName,
          validator: { $jsonSchema: updated },
          validationLevel,
          validationAction,
        })
      }
    }
  }

  const collection = db.collection(collectionName)

  // ── createIndex ────────────────────────────────────────────────
  if (ops.createIndex && ops.createIndex.length > 0) {
    const models = buildCreateIndexModels(ops.createIndex)
    for (const model of models) {
      await collection.createIndex(model.key, model.options)
    }
  }

  // ── alterIndex (drop + recrear) ────────────────────────────────
  if (ops.alterIndex && ops.alterIndex.length > 0) {
    const names = extractIndexNames(ops.alterIndex)
    for (const name of names) {
      try {
        await collection.dropIndex(name)
      } catch {
        // si no existe el indice simplemente continua
      }
    }
    const models = buildCreateIndexModels(ops.alterIndex)
    for (const model of models) {
      await collection.createIndex(model.key, model.options)
    }
  }

  // ── dropIndex ──────────────────────────────────────────────────
  if (ops.dropIndex && ops.dropIndex.length > 0) {
    for (const name of ops.dropIndex) {
      await collection.dropIndex(name)
    }
  }

  // ── indexModel (raw mongo) ─────────────────────────────────────
  if (ops.indexModel && ops.indexModel.length > 0) {
    const models = buildRawIndexModels(ops.indexModel)
    for (const model of models) {
      await collection.createIndex(model.key, model.options)
    }
  }
}

// ================================================================
//              DOWN
// ================================================================

export async function runMongoDown(db: Db, collectionName: string, ops: CollectionOperations): Promise<void> {
  const validationLevel = ops.validationLevel ?? "off"
  const validationAction = ops.validationAction ?? "error"
  const additionalProperties = ops.additionalProperties ?? false

  // ── dropCollection ─────────────────────────────────────────────
  if (ops.dropCollection) {
    await db.collection(collectionName).drop()
    return
  }

  const collection = db.collection(collectionName)

  // ── dropIndex ──────────────────────────────────────────────────
  if (ops.dropIndex && ops.dropIndex.length > 0) {
    for (const name of ops.dropIndex) {
      await collection.dropIndex(name)
    }
  }

  // ── alterIndex ─────────────────────────────────────────────────
  if (ops.alterIndex && ops.alterIndex.length > 0) {
    const names = extractIndexNames(ops.alterIndex)
    for (const name of names) {
      try {
        await collection.dropIndex(name)
      } catch {
        // si no existe continua
      }
    }
    const models = buildCreateIndexModels(ops.alterIndex)
    for (const model of models) {
      await collection.createIndex(model.key, model.options)
    }
  }

  // ── createIndex en down (restaurar indices eliminados en up) ───
  if (ops.createIndex && ops.createIndex.length > 0) {
    const models = buildCreateIndexModels(ops.createIndex)
    for (const model of models) {
      await collection.createIndex(model.key, model.options)
    }
  }

  // ── dropField ──────────────────────────────────────────────────
  if (ops.dropField && ops.dropField.length > 0) {
    const info = await db.listCollections({ name: collectionName }, { nameOnly: false }).toArray()
    const existingSchema = info[0]?.options?.validator?.$jsonSchema
    if (existingSchema) {
      const updated = dropFromJsonSchema(existingSchema, ops.dropField)
      await db.command({
        collMod: collectionName,
        validator: { $jsonSchema: updated },
        validationLevel,
        validationAction,
      })
    }
  }

  // ── alterField en down (restaurar schema anterior) ─────────────
  if (ops.alterField && ops.alterField.length > 0) {
    const info = await db.listCollections({ name: collectionName }, { nameOnly: false }).toArray()
    const existingSchema = info[0]?.options?.validator?.$jsonSchema
    if (existingSchema) {
      const merged = mergeJsonSchema(existingSchema, ops.alterField, additionalProperties)
      await db.command({
        collMod: collectionName,
        validator: { $jsonSchema: merged },
        validationLevel,
        validationAction,
      })
    } else {
      const schema = buildJsonSchema(ops.alterField, additionalProperties)
      await db.command({
        collMod: collectionName,
        validator: { $jsonSchema: schema },
        validationLevel,
        validationAction,
      })
    }
  }

  // ── createField en down (restaurar campos eliminados en up) ────
  if (ops.createField && ops.createField.length > 0) {
    const info = await db.listCollections({ name: collectionName }, { nameOnly: false }).toArray()
    const existingSchema = info[0]?.options?.validator?.$jsonSchema
    if (existingSchema) {
      const merged = mergeJsonSchema(existingSchema, ops.createField, additionalProperties)
      await db.command({
        collMod: collectionName,
        validator: { $jsonSchema: merged },
        validationLevel,
        validationAction,
      })
    }
  }
}
