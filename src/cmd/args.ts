import { argv } from "bun"

import { actions, migrationFlags } from "./actions"
import { printPrimary } from "./cmd"
import {
  getName,
  type NameConvention,
  promptMigrationFlags,
  promptName,
  promptRunFlags,
  promptTargets,
} from "./prompts"

export interface Args {
  targets: string[]
  name: NameConvention | undefined
  flags: string[]
}

export async function getArgs(): Promise<Args> {
  printPrimary(`ARGS: ${argv}`)

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

  if (inputs.length < 2 || !inputs[1]) {
    name = await promptName()
  } else {
    name = getName(inputs[1])
  }

  if (inputs.length > 2) {
    flags.push(...inputs.slice(2))
  }

  if (targets[0] === "project") {
    return { targets, name, flags }
  }

  // si el usuario escribe el comando template con flags se pasan a targets
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
