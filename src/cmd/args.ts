import { checkbox, input, select } from "@inquirer/prompts"
import { argv } from "bun"
import type { Args } from "../types/Args"
import type { NameConvention } from "../types/NameConvention"
import {
  pluralizeIdentifier,
  toCamelCase,
  toKebabCase,
  toPascalCase,
  toSnakeCase,
  toText,
  toTitleCase,
} from "../utils/str"
import { actions, migrationFlags } from "./actions"

export async function getArgs(): Promise<Args> {
  const inputs = argv.slice(2)
  let targets: string[]
  let name: NameConvention
  const flags: string[] = []

  if (inputs.length < 1 || !inputs[0]) {
    targets = await promptTargets()
  } else {
    targets = getTargets(inputs[0])
  }

  if (targets[0] === "help" || targets[0] === "version") {
    return { targets, name: undefined, flags: [] }
  }

  if (targets[0] === "run") {
    // si paso por el promp de targets este toma los flags
    if (targets.length > 1) {
      return { targets, name: undefined, flags: await getRunFlags(targets.slice(1)) }
    }
    // cuando el usuario escribe el comando run con argumentos
    return { targets, name: undefined, flags: await getRunFlags(inputs.slice(1)) }
  }

  if (targets[0] === "project") {
    if (inputs.length < 2 || !inputs[1]) {
      name = await promptProjectName()
    } else {
      if (!isProjectNameValidFormat(inputs[1])) {
        throw new Error("Format must be gitUser/projectName")
      }
      name = getName(inputs[1])
    }
    return { targets, name, flags }
  }

  if (inputs.length < 2 || !inputs[1]) {
    name = await promptTemplateName()
  } else {
    name = getName(inputs[1])
  }

  // si el usuario escribe el comando template con flags se pasan a targets
  if (inputs.length > 2) {
    flags.push(...inputs.slice(2))
  }

  const result = new Set<string>()

  // escribo los targets en el set
  for (const v of targets) {
    result.add(v)
  }

  for (const key of flags) {
    if (!(key in actions)) {
      throw new Error(`Invalid flag: ${key}`)
    }
    const values = actions[key as keyof typeof actions]
    for (const v of values) {
      result.add(v)
    }
  }
  targets = Array.from(result)

  return { targets, name, flags: [] }
}

function getTargets(inputTarget: string): string[] {
  if (!(inputTarget in actions)) {
    throw new Error(`Invalid target: ${inputTarget}`)
  }

  return actions[inputTarget as keyof typeof actions]
}

async function getRunFlags(input: string[]): Promise<string[]> {
  const flags: string[] = []

  if (input.length === 0) {
    const runFlags = await promptRunFlags()
    // flags.push(runFlags)
    input.push(runFlags)
  }

  if (input[0] === "migration") {
    flags.push("migration")
    if (!input[1]) {
      flags.push(...(await promptMigrationFlags()))
      return flags
    }

    if (!migrationFlags.includes(input[1])) {
      throw new Error(`Invalid migration flag: ${input[1]}`)
    }
    flags.push(input[1])

    if (input[1] === "rollback") {
      if (input[2]) {
        const step = Number(input[2])
        if (!Number.isInteger(step) || step < 1) {
          throw new Error(`Invalid rollback step number: ${input[2]}`)
        }
        flags.push(input[2])
      } else {
        flags.push("1")
      }
    }
    return flags
  }

  if (input[0] === "seed") {
    flags.push("seed")
    if (input.length > 1) {
      flags.push(...input.slice(1))
    }
    return flags
  }

  if (input[0] === "dev") {
    flags.push("dev")
    if (input.length > 1) {
      flags.push(...input.slice(1))
    }
    return flags
  }

  if (input[0] === "build") {
    flags.push("build")
    if (input.length > 1) {
      flags.push(...input.slice(1))
    }
    return flags
  }

  throw new Error(`Invalid run flag: ${input[0]}`)
}

// ================================================================
//                            PROMPTS
// ================================================================

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
        { name: "model       database model", value: "model" },
        { name: "migration   database schema", value: "migration" },
        { name: "seed        seed data", value: "seed" },
        { name: "repository  data access layer", value: "repository" },
        { name: "resource    API resource", value: "resource" },

        { name: "view        page + ts + css", value: "view" },
        { name: "page        gtml page", value: "page" },
        { name: "component   reusable gtml component", value: "component" },
        { name: "layout      layout structure", value: "layout" },

        { name: "controller  request handler", value: "controller" },
        { name: "middleware  request filter", value: "middleware" },
        { name: "policy      authorization rules", value: "policy" },
        { name: "route       route definition", value: "route" },
        { name: "service     business logic", value: "service" },
        { name: "validator   input validation", value: "validator" },

        { name: "data        model + migration + seed", value: "data" },
        { name: "handler     controller + policy + route + validator", value: "handler" },

        { name: "ui          page + component + layout + ts + css", value: "ui" },
        { name: "api         backend layer data + handler", value: "api" },
        { name: "mvc         full stack (model-view-controller)", value: "mvc" },
        { name: "all         everything", value: "all" },
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

export async function promptTemplateName(): Promise<NameConvention> {
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

export async function promptProjectName(): Promise<NameConvention> {
  const raw = await input({
    message: "Enter project (gitUser/projectName)",
    validate: (value) => {
      const v = value.trim()

      if (!v) {
        return "Project is required"
      }

      if (!isProjectNameValidFormat(v)) {
        return "Format must be gitUser/projectName"
      }

      return true
    },
  })

  return getName(raw.trim())
}

export function isProjectNameValidFormat(name: string): boolean {
  // permite: letras, numeros, guiones, guiones bajos, puntos, espacios y slash
  if (!/^[a-zA-Z0-9_\-./ ]+$/.test(name)) {
    return false
  }
  const parts = name.split("/")

  if (parts.length !== 2) {
    return false
  }
  if (!parts[0] || !parts[1]) {
    return false
  }
  if (parts[0].includes(" ")) {
    return false
  }
  return true
}

export function getName(inputName: string): NameConvention {
  const singular = inputName
  const plural = pluralizeIdentifier(inputName)

  let projectName: string = ""
  let gitUser: string = ""
  const [user = "", name = ""] = inputName.split("/")
  if (user && name) {
    projectName = toKebabCase(name)
    gitUser = user.toLowerCase()
  }

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
    project: projectName,
    gitUser: gitUser,
    gomod: `${gitUser}/${projectName}`,
  }
}
