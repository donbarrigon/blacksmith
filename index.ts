import { getArgs } from "./src/utils/args"

const args = await getArgs()

for (const target of args.targets) {
  switch (target) {
    case "version": {
      const { version } = await import("./src/cmd/help")
      version()
      break
    }

    case "help": {
      const { help } = await import("./src/cmd/help")
      help()
      break
    }

    case "project": {
      const { createProject } = await import("./src/cmd/project")
      await createProject(args)
      break
    }

    case "run": {
      switch (args.flags[0]) {
        case "migration": {
          const { runMigration } = await import("./src/cmd/run/migration")
          await runMigration(args)
          break
        }

        default:
          break
      }
      break
    }

    default:
      break
  }
}
