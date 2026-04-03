import { checkbox, input, select } from "@inquirer/prompts"
import {
  pluralizeIdentifier,
  toCamelCase,
  toKebabCase,
  toPascalCase,
  toSnakeCase,
  toText,
  toTitleCase,
} from "../utils/str"
import { actions } from "./actions"

// NameVariants
export interface NameConvention {
  // input: service_item
  singular: {
    snake: string // service_item
    camel: string // serviceItem
    pascal: string // ServiceItem
    kebab: string // service-item
    text: string // service item
    title: string // Service Item
  }

  plural: {
    snake: string // service_items
    camel: string // serviceItems
    pascal: string // ServiceItems
    kebab: string // service-items
    text: string // service items
    title: string // Service Items
  }

  original: string // service_item
}

export async function promptTargets(): Promise<string[]> {
  const main = await select({
    message: "Select an action",
    choices: [
      { name: "Create template", value: "template" },
      { name: "Run migration", value: "run migration" },
      { name: "Run seeds", value: "run seed" },
      { name: "Create project", value: "project" },
      { name: "Version", value: "version" },
      { name: "Help", value: "help" },
    ],
  })

  // TEMPLATE (multi select)
  if (main === "template") {
    const selected = await checkbox({
      message: "Select templates to generate",
      choices: [
        { name: "model - database model", value: "model" },
        { name: "migration - database schema", value: "migration" },
        { name: "seed - seed data", value: "seed" },
        { name: "repository - data access layer", value: "repository" },
        { name: "resource - API resource", value: "resource" },

        { name: "view - page + ts + css", value: "view" },
        { name: "page - UI page", value: "page" },
        { name: "component - reusable UI", value: "component" },
        { name: "layout - layout structure", value: "layout" },

        { name: "controller - request handler", value: "controller" },
        { name: "middleware - request filter", value: "middleware" },
        { name: "policy - authorization rules", value: "policy" },
        { name: "route - route definition", value: "route" },
        { name: "service - business logic", value: "service" },
        { name: "validator - input validation", value: "validator" },

        { name: "data - model + migration + seed", value: "data" },
        { name: "handler - controller + policy + route + validator", value: "handler" },

        { name: "ui - page + component + layout + ts + css", value: "ui" },
        { name: "api - backend layer", value: "api" },
        { name: "mvc - full stack (model-view-controller)", value: "mvc" },
        { name: "all - everything", value: "all" },
      ],
      validate: (answers) => {
        if (answers.length === 0) {
          return "You must select at least one option"
        }
        return true
      },
    })

    const result = new Set<string>()

    for (const key of selected) {
      const values = actions[key as keyof typeof actions]
      for (const v of values) {
        result.add(v)
      }
    }

    return Array.from(result)
  }

  if (main === "run migration") {
    return ["run", "migration"]
  }

  if (main === "run seed") {
    return ["run", "seed"]
  }

  // OTROS (retorno directo)
  return [main]
}

export async function promptRunFlags(): Promise<string> {
  const choice = await select({
    message: "Select what you want to run",
    choices: [
      {
        name: "Migration - run database migrations",
        value: "migration",
      },
      {
        name: "Seed - run database seeders",
        value: "seed",
      },
      {
        name: "Dev - run app in development mode",
        value: "dev",
      },
      {
        name: "Build - compile the application",
        value: "build",
      },
    ],
  })

  return choice
}

export async function promptMigrationFlags(): Promise<string[]> {
  const action = await select({
    message: "Select migration action",
    default: "up",
    choices: [
      { name: "up - run pending migrations", value: "up" },
      { name: "rollback - revert last batch", value: "rollback" },
      { name: "reset - revert all migrations", value: "reset" },
      { name: "refresh - rollback and re-run", value: "refresh" },
      { name: "fresh - drop all and re-run", value: "fresh" },
    ],
  })

  // 🔹 Caso especial: rollback con step
  if (action === "rollback") {
    const raw = await input({
      message: "Enter rollback steps",
      default: "1",
      validate: (value) => {
        if (!/^\d+$/.test(value) || Number(value) < 1) {
          return "Step must be a positive integer"
        }
        return true
      },
    })

    return ["rollback", raw]
  }

  return [action]
}

export async function promptName(): Promise<NameConvention> {
  const raw = await input({
    message: "Enter name",
    validate: (value) => {
      if (!value.trim()) {
        return "Name is required"
      }

      // permite: letras, numeros, guiones, guiones bajos, puntos y espacios
      if (!/^[a-zA-Z0-9_\-. ]+$/.test(value)) {
        return "Invalid name format"
      }

      return true
    },
  })

  return getName(raw.trim())
}

export function getName(inputName: string): NameConvention {
  const singular = inputName
  const plural = pluralizeIdentifier(inputName)

  return {
    singular: {
      snake: toSnakeCase(singular),
      camel: toCamelCase(singular),
      pascal: toPascalCase(singular),
      kebab: toKebabCase(singular),
      text: toText(singular),
      title: toTitleCase(singular),
    },
    plural: {
      snake: toSnakeCase(plural),
      camel: toCamelCase(plural),
      pascal: toPascalCase(plural),
      kebab: toKebabCase(plural),
      text: toText(plural),
      title: toTitleCase(plural),
    },
    original: inputName,
  }
}
