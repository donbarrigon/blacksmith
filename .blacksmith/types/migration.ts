export interface Migration {
  // estructura
  collection: string
  fields?: Field[]
  indexes?: Index[]

  // configuracion
  validationLevel?: "strict" | "moderate" | "off" // solo mongo
  validationAction?: "error" | "warn" // solo mongo
  additionalProperties?: boolean // solo mongo
  engine?: EngineType // solo mysql

  // acciones
  dropCollection?: boolean // elimina la coleccion
  dropField?: string[] // nombre del field a eliminar
  dropIndex?: string[] | string[][] // string [] nombre del index // string [][] array con los fileds del index a eliminar
  alterField?: Field[] // modifica los campos
  alterIndex?: Index[] // modifica los indices
}

export interface Field {
  name: string | [string, string] // [string, string] es solo para alter [oldName, newName]
  type: FieldType
  length?: number
  nullable?: boolean
  default?: string | number | boolean | string[] | number[] | boolean[] | null
  autoIncrement?: boolean // solo mysql y postgres
  unsigned?: boolean
  precision?: number
  scale?: number
  description?: string
  values?: string[] // se define el esquema de los type enum
  jsonSchema?: Field[] // se define el esquema de los type json
  items?: ArrayItems // se define el esquema de los type array
}

export interface ArrayItems {
  type: FieldType
  nullable?: boolean
  description?: string
  values?: string[]
  jsonSchema?: Field[]
}

export interface Index {
  field: string | string[]
  type?: IndexType
  name?: string
  primary?: boolean
  unique?: boolean
  sparse?: boolean
  nullable?: boolean
  order?: 1 | -1
  background?: boolean // solo mongo legacy
  expireAfterSeconds?: number
  collation?: object
  description?: string
  foreign?: Foreign // solo motores sql
}

export interface Foreign {
  auto?: boolean // toma el field como referencia para rellenar collection y field
  collection?: string
  field?: string
  onUpdate?: "cascade" | "restrict" | "set_null" | "set_default" | "no_action"
  onDelete?: "cascade" | "restrict" | "set_null" | "set_default" | "no_action"
}

export type IndexType = "index" | "text" | "hashed" | "2dsphere" | "2d" | "wildcard"

export type FieldType =
  | "int8"
  | "int16"
  | "int"
  | "int64"
  | "uint8"
  | "uint16"
  | "uint"
  | "uint64"
  | "float"
  | "double"
  | "decimal"
  | "string"
  | "text"
  | "char"
  | "bool"
  | "date"
  | "datetime"
  | "timestamp"
  | "objectId"
  | "uuid"
  | "json"
  | "array"
  | "enum"
  | "binary"
  | "inet"
  | "money"

export type EngineType = "InnoDB" | "MyISAM" | "MEMORY" | "CSV" | "ARCHIVE" | "BLACKHOLE" | "NDB"
