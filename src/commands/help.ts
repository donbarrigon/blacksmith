// import { readFile } from "node:fs/promises"
// import { join } from "node:path"
import { color, style } from "../cmd/print"

export function version(): void {
  const c = color.text
  const r = color.reset
  const bold = style.bold
  const dim = style.dim

  console.log(`
${bold}${c.primary}  ███████╗ ██████╗ ██████╗  ██████╗ ███████╗${r}
${bold}${c.primary}  ██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝${r}
${bold}${c.primary}  █████╗  ██║   ██║██████╔╝██║  ███╗█████╗  ${r}
${bold}${c.primary}  ██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝  ${r}
${bold}${c.primary}  ██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗${r}
${bold}${c.primary}  ╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝${r}
${dim}                                       v0.7.0${r}
`)
}

export function help(): void {
  const c = color.text
  const r = color.reset
  const bold = style.bold
  const dim = style.dim
  const italic = style.italic

  console.log(
    `
${bold}${c.primary}  ███████╗ ██████╗ ██████╗  ██████╗ ███████╗${r}
${bold}${c.primary}  ██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝${r}
${bold}${c.primary}  █████╗  ██║   ██║██████╔╝██║  ███╗█████╗  ${r}
${bold}${c.primary}  ██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝  ${r}
${bold}${c.primary}  ██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗${r}
${bold}${c.primary}  ╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝${r}
${dim}  A code generator CLI${r}

${bold}  USAGE${r}
  ${c.secondary}fg${r} ${c.info}<command>${r} ${c.warning}<name>${r} ${dim}[flags]${r}

${bold}  TEMPLATES${r}  ${dim}fg <template> <name>${r}
  ${c.gray}─────────────────────────────────────────────${r}
  ${c.info}model${r}        ${c.warning}<name>${r}   ${dim}database model${r}
  ${c.info}migration${r}    ${c.warning}<name>${r}   ${dim}database schema${r}
  ${c.info}seed${r}         ${c.warning}<name>${r}   ${dim}seed data${r}
  ${c.info}repository${r}   ${c.warning}<name>${r}   ${dim}data access layer${r}
  ${c.info}resource${r}     ${c.warning}<name>${r}   ${dim}API resource${r}
  ${c.info}page${r}         ${c.warning}<name>${r}   ${dim}gtml page${r}
  ${c.info}component${r}    ${c.warning}<name>${r}   ${dim}reusable gtml component${r}
  ${c.info}layout${r}       ${c.warning}<name>${r}   ${dim}layout structure${r}
  ${c.info}controller${r}   ${c.warning}<name>${r}   ${dim}request handler${r}
  ${c.info}middleware${r}   ${c.warning}<name>${r}   ${dim}request filter${r}
  ${c.info}policy${r}       ${c.warning}<name>${r}   ${dim}authorization rules${r}
  ${c.info}route${r}        ${c.warning}<name>${r}   ${dim}route definition${r}
  ${c.info}service${r}      ${c.warning}<name>${r}   ${dim}business logic${r}
  ${c.info}validator${r}    ${c.warning}<name>${r}   ${dim}input validation${r}

${bold}  BUNDLES${r}  ${dim}fg <bundle> <name> [flags]${r}
  ${c.gray}─────────────────────────────────────────────${r}
  ${c.info}view${r}         ${c.warning}<name>${r}   ${dim}page + ts + css${r}
  ${c.info}data${r}         ${c.warning}<name>${r}   ${dim}model + migration + seed${r}
  ${c.info}handler${r}      ${c.warning}<name>${r}   ${dim}controller + policy + route + validator${r}
  ${c.info}ui${r}           ${c.warning}<name>${r}   ${dim}page + component + layout + ts + css${r}
  ${c.info}api${r}          ${c.warning}<name>${r}   ${dim}data + handler${r}
  ${c.info}mvc${r}          ${c.warning}<name>${r}   ${dim}full stack model-view-controller${r}
  ${c.info}all${r}          ${c.warning}<name>${r}   ${dim}everything${r}

${bold}  OTHER COMMANDS${r}
  ${c.gray}─────────────────────────────────────────────${r}
  ${c.info}project${r}      ${c.warning}<gitUser/projectName>${r}        ${dim}create a new project${r}
  ${c.info}run${r}          ${c.warning}migration${r} ${dim}[up|rollback|reset|refresh|fresh]${r}
  ${c.info}run${r}          ${c.warning}seed${r}
  ${c.info}run${r}          ${c.warning}dev${r}
  ${c.info}run${r}          ${c.warning}build${r}
  ${c.info}version${r}
  ${c.info}help${r}

${bold}  EXAMPLES${r}
  ${c.gray}─────────────────────────────────────────────${r}
  ${c.secondary}fg${r} ${c.info}model${r} ${c.warning}User${r}
  ${c.secondary}fg${r} ${c.info}mvc${r} ${c.warning}Post${r}
  ${c.secondary}fg${r} ${c.info}api${r} ${c.warning}Product${r}
  ${c.secondary}fg${r} ${c.info}project${r} ${c.warning}donbarrigon/my-app${r}
  ${c.secondary}fg${r} ${c.info}run migration${r} ${c.warning}rollback${r} ${dim}3${r}

${bold}  INTERACTIVE MODE${r}
  ${c.gray}─────────────────────────────────────────────${r}
  ${italic}${dim}Running fg without arguments or with incomplete commands
  launches an interactive prompt to guide you step by step.${r}
  ${c.success}fg${r}
`,
  )
}
