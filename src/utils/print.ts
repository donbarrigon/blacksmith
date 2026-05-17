export const color = {
  reset: "\x1b[0m",

  text: {
    primary: "\x1b[32m", // magenta \x1b[95m
    secondary: "\x1b[94m", // magenta
    info: "\x1b[96m", // cyan
    warning: "\x1b[93m", // amarillo brillante
    danger: "\x1b[91m", // rojo
    success: "\x1b[92m", // verde
    gray: "\x1b[90m", // gris
  },

  bg: {
    primary: "\x1b[43m",
    secondary: "\x1b[45m",
    info: "\x1b[46m",
    warning: "\x1b[103m",
    danger: "\x1b[41m",
    success: "\x1b[42m",
  },
}

export const style = {
  bold: "\x1b[1m",
  dim: "\x1b[2m",
  italic: "\x1b[3m",
  underline: "\x1b[4m",
} as const

let textColor: string = color.reset

function format(text: string, colorCode: string, emoji?: string) {
  const prefix = emoji ? `${emoji} ` : ""
  return `${colorCode}${prefix} ${text}${textColor}\n`
}

export function setTextColor(txtColor: keyof typeof color.text) {
  textColor = color.text[txtColor] || color.reset
}

export function resetTextColor() {
  textColor = color.reset
}

export function print(text: string) {
  console.log(text)
}

export function println(text: string) {
  console.log(`${text}\n`)
}

export function printError(text: string) {
  console.error(format(text, color.text.danger, "❌"))
}

export function printWarning(text: string) {
  console.warn(format(text, color.text.warning, "⚠️"))
}

export function printInfo(text: string) {
  console.info(format(text, color.text.info, "ℹ️"))
}

export function printSuccess(text: string) {
  console.log(format(text, color.text.success, "✅"))
}

export function printDebug(text: string) {
  console.debug(format(text, color.text.gray, "🐛"))
}

export function printPrimary(text: string) {
  console.log(`\n  ${format(text, color.text.primary)}\n`)
  // 🔥
}

export function printSecondary(text: string) {
  console.log(`  ${format(text, color.text.secondary)}`)
  // ✨
}
