export const irregularPlurals: Record<string, string> = {
  person: "people",
  child: "children",
  foot: "feet",
  tooth: "teeth",
  mouse: "mice",
  man: "men",
  woman: "women",
  ox: "oxen",
  cactus: "cacti",
  focus: "foci",
  analysis: "analyses",
  thesis: "theses",
  crisis: "crises",
  diagnosis: "diagnoses",
  appendix: "appendices",
  vertex: "vertices",
  index: "indices",
  matrix: "matrices",
  axis: "axes",
  basis: "bases",
  fungus: "fungi",
  radius: "radii",
  alumnus: "alumni",
  curriculum: "curricula",
  datum: "data",
  medium: "media",
  forum: "fora",
  bacterium: "bacteria",
  syllabus: "syllabi",
  criterion: "criteria",
  aquarium: "aquaria",
  stadium: "stadia",
  stimulus: "stimuli",
  die: "dice",
  formula: "formulae",
  genus: "genera",
  bison: "bison",
  deer: "deer",
  sheep: "sheep",
  salmon: "salmon",
  aircraft: "aircraft",
  series: "series",
  species: "species",
  fish: "fish",
  trousers: "trousers",
  scissors: "scissors",
  clothes: "clothes",
  news: "news",
}

function isVowel(char: string): boolean {
  return "aiueo".includes(char)
}

/** Separa una cadena en palabras sin importar el formato original */
function splitWords(s: string): string[] {
  if (!s) return []

  s = s.replaceAll("-", " ").replaceAll("_", " ")

  let result = ""
  for (let i = 0; i < s.length; i++) {
    const char = s[i] as string // espara que el compilador no de error en la linea 68
    const prev = s[i - 1] ?? ""
    if (
      i > 0 &&
      char === char.toUpperCase() &&
      char !== char.toLowerCase() &&
      (prev === prev.toLowerCase() || /\d/.test(prev))
    ) {
      result += " "
    }
    result += char
  }

  return result.toLowerCase().split(/\s+/).filter(Boolean)
}

/** Capitaliza la primera letra de una palabra */
function capitalize(s: string): string {
  if (!s) return ""
  return s.charAt(0).toUpperCase() + s.slice(1)
}

/** Convierte a snake_case */
export function toSnakeCase(s: string): string {
  return splitWords(s).join("_")
}

/** Convierte a camelCase */
export function toCamelCase(s: string): string {
  const words = splitWords(s)
  return words.map((w, i) => (i === 0 ? w : capitalize(w))).join("")
}

/** Convierte a PascalCase */
export function toPascalCase(s: string): string {
  return splitWords(s).map(capitalize).join("")
}

/** Convierte a kebab-case */
export function toKebabCase(s: string): string {
  return splitWords(s).join("-")
}

const minorWords = new Set([
  "a",
  "an",
  "the",
  "and",
  "but",
  "or",
  "nor",
  "for",
  "yet",
  "so",
  "at",
  "by",
  "in",
  "of",
  "on",
  "to",
  "up",
  "as",
])

/** Convierte a Title Case (cada palabra con inicial mayúscula, separadas por espacios) */
export function toTitleCase(s: string): string {
  const words = splitWords(s)
  return words
    .map((word, i) => {
      const isFirst = i === 0
      const isLast = i === words.length - 1
      if (!isFirst && !isLast && minorWords.has(word)) return word
      return capitalize(word)
    })
    .join(" ")
}

/** Convierte a text (todo en minúsculas, separado por espacios) */
export function toText(s: string): string {
  return splitWords(s).join(" ")
}

/** Convierte un identificador a una variable corta usando las iniciales de cada palabra */
export function toVar(s: string): string {
  if (!s) return ""
  return splitWords(s)
    .map((w) => w.charAt(0))
    .join("")
}

/** Pluraliza una palabra según las reglas estándar del inglés */
export function pluralize(word: string): string {
  if (!word) return ""

  if (irregularPlurals[word]) return irregularPlurals[word] as string

  if (word.endsWith("y")) {
    if (word.length > 1 && isVowel(word[word.length - 2] as string)) {
      return `${word}s`
    }
    return `${word.slice(0, -1)}ies`
  }

  if (word.endsWith("s") || word.endsWith("x") || word.endsWith("z") || word.endsWith("ch") || word.endsWith("sh")) {
    return `${word}es`
  }

  if (word.endsWith("fe")) return `${word.slice(0, -2)}ves`
  if (word.endsWith("f")) return `${word.slice(0, -1)}ves`

  return `${word}s`
}

type IdentifierFormat = "snake" | "camel" | "pascal" | "kebab" | "unknown"

/** Detecta el formato del identificador */
function detectFormat(s: string): IdentifierFormat {
  if (s.includes("_")) return "snake"
  if (s.includes("-")) return "kebab"

  for (let i = 0; i < s.length; i++) {
    if (s[i] !== s[i]?.toLowerCase()) {
      return i === 0 ? "pascal" : "camel"
    }
  }

  return "snake"
}

/**
 * Pluraliza la última palabra de un identificador
 * manteniendo el formato original (snake_case, camelCase, PascalCase, kebab-case)
 */
export function pluralizeIdentifier(s: string): string {
  if (!s) return ""

  const format = detectFormat(s)
  const words = splitWords(s)
  if (!words.length) return s

  words[words.length - 1] = pluralize(words[words.length - 1] as string)

  const joined = words.join(" ")
  switch (format) {
    case "snake":
      return toSnakeCase(joined)
    case "camel":
      return toCamelCase(joined)
    case "pascal":
      return toPascalCase(joined)
    case "kebab":
      return toKebabCase(joined)
    default:
      return joined
  }
}
