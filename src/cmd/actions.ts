export const actions = {
  // contruccion de templates
  model: ["model"],
  migration: ["migration"],
  seed: ["seed"],
  repository: ["repository"],
  resource: ["resource"],

  view: ["page", "ts", "css", "view"],
  page: ["page"],
  component: ["component"],
  layout: ["layout"],

  controller: ["controller"],
  middleware: ["middleware"],
  policy: ["policy"],
  route: ["route"],
  service: ["service"],
  validator: ["validator"],

  data: ["model", "migration", "seed", "data"],
  handler: ["controller", "policy", "route", "validator", "handler"],

  ui: ["page", "component", "layout", "ts", "css", "ui"],
  api: ["model", "migration", "seed", "controller", "policy", "route", "validator", "api"],
  mvc: ["model", "migration", "seed", "page", "ts", "css", "controller", "policy", "route", "validator", "mvc"],
  all: [
    "model",
    "migration",
    "seed",
    "repository",
    "resource",
    "page",
    "component",
    "layout",
    "ts",
    "css",
    "controller",
    "middleware",
    "policy",
    "route",
    "service",
    "validator",
    "view",
    "data",
    "handler",
    "ui",
    "api",
    "mvc",
    "all",
  ],

  // otras acciones
  help: ["help"], //       fg help
  version: ["version"], // fg version
  project: ["project"], // fg project [project_name]
  run: ["run"], //         fg run migration // fg run seed // fg run migration rollback
}

export const migrationFlags = ["up", "rollback", "reset", "refresh", "fresh"]
