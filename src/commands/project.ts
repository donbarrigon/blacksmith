import { readdir, readFile, rm, writeFile } from "node:fs/promises"
import { join } from "node:path"
import type { Args } from "../types/Args"

const ORIGINAL_MODULE = "github.com/donbarrigon/forge"

export async function createProject(args: Args): Promise<void> {
  await downloadStarterKit(args)
  await renameProject(args)
}

async function downloadStarterKit(args: Args): Promise<void> {
  if (!args.name) {
    throw new Error("Name is required to create project")
  }
  const repo = "https://github.com/donbarrigon/forge.git"

  const proc = Bun.spawn(["git", "clone", "--depth=1", repo, args.name.project])

  const exitCode = await proc.exited

  if (exitCode !== 0) {
    throw new Error("Failed to clone repository")
  }

  await rm(`${args.name.project}/.git`, { recursive: true, force: true })
}

async function renameProject(args: Args): Promise<void> {
  if (!args.name) {
    throw new Error("Name is required to rename project")
  }

  const { project, gomod } = args.name
  const newModule = `github.com/${gomod}`

  // package.json
  const packageJsonPath = join(project, "package.json")
  const packageJson = JSON.parse(await readFile(packageJsonPath, "utf-8"))
  packageJson.name = project
  await writeFile(packageJsonPath, JSON.stringify(packageJson, null, 2))

  // go.mod
  const goModPath = join(project, "go.mod")
  const goMod = await readFile(goModPath, "utf-8")
  await writeFile(goModPath, goMod.replace(ORIGINAL_MODULE, newModule))

  // todos los .go en internal recursivamente
  await replaceInGoFiles(join(project, "internal"), ORIGINAL_MODULE, newModule)
}

async function replaceInGoFiles(dir: string, from: string, to: string): Promise<void> {
  const entries = await readdir(dir, { withFileTypes: true, recursive: true })

  await Promise.all(
    entries
      .filter((entry) => entry.isFile() && entry.name.endsWith(".go"))
      .map(async (entry) => {
        const fullPath = join(entry.parentPath, entry.name)
        const content = await readFile(fullPath, "utf-8")
        if (content.includes(from)) {
          await writeFile(fullPath, content.replaceAll(from, to))
        }
      }),
  )
}
