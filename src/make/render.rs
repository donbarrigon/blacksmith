//! Contenido de los archivos que genera `make`.

use super::spec::{FieldLine, Model, rust_ident};

/// Migración que crea la colección con los campos dados.
pub fn migration_create(id: &str, struct_name: &str, coll: &str, lines: &[FieldLine]) -> String {
    let code: Vec<&str> = lines.iter().map(|l| l.code.as_str()).collect();
    let body = if code.is_empty() {
        "            col.id();\n            col.timestamps();\n".to_string()
    } else {
        code.iter().map(|c| format!("            col.{c};\n")).collect()
    };
    let imports = migration_imports(&code.join("\n"), "Validation");
    format!(
        r#"//! Generada con blacksmith.

use ironforge::ForgeError;
use ironforge::migration::{{{imports}}};

pub struct {struct_name};

impl Migration for {struct_name} {{
    fn migration_id(&self) -> &'static str {{
        "{id}"
    }}

    fn coll_name(&self) -> &'static str {{
        "{coll}"
    }}

    async fn up(&self) -> Result<Schema, ForgeError> {{
        Ok(Schema::create_collection(|col| {{
{body}        }})
        .validation(Validation::Auto))
    }}

    async fn down(&self) -> Result<Schema, ForgeError> {{
        Ok(Schema::drop_collection())
    }}
}}
"#
    )
}

/// Migración que modifica una colección existente (para completar a mano).
pub fn migration_alter(id: &str, struct_name: &str, coll: &str) -> String {
    format!(
        r#"//! Generada con blacksmith.

use ironforge::ForgeError;
use ironforge::migration::{{Migration, Schema, Validation}};

pub struct {struct_name};

impl Migration for {struct_name} {{
    fn migration_id(&self) -> &'static str {{
        "{id}"
    }}

    fn coll_name(&self) -> &'static str {{
        "{coll}"
    }}

    async fn up(&self) -> Result<Schema, ForgeError> {{
        // Cambia `_col` por `col` y escribe solo los cambios, por ejemplo:
        //   col.string("phone").optional();
        //   col.string("name").max(100).change();
        //   col.rename_field("a", "b");
        Ok(Schema::alter_collection(|_col| {{}})
            .validation(Validation::Preserve))
    }}

    async fn down(&self) -> Result<Schema, ForgeError> {{
        Ok(Schema::revert())
    }}
}}
"#
    )
}

pub fn seed(id: &str, struct_name: &str) -> String {
    format!(
        r#"//! Generado con blacksmith.

use ironforge::ForgeError;
use ironforge::migration::Seeder;

pub struct {struct_name};

impl Seeder for {struct_name} {{
    fn seed_id(&self) -> &'static str {{
        "{id}"
    }}

    async fn run(&self) -> Result<(), ForgeError> {{
        // Inserta los datos, por ejemplo con un modelo: User {{ .. }}.create().await?;
        Ok(())
    }}
}}
"#
    )
}

pub fn model(struct_name: &str, coll: &str, migration_module: &str, m: &Model) -> String {
    let bson: Vec<&str> = m.bson.iter().copied().collect();
    let bson_use = match bson.as_slice() {
        [one] => format!("use mongodb::bson::{one};\n"),
        many => format!("use mongodb::bson::{{{}}};\n", many.join(", ")),
    };
    let mut fields = String::new();
    for f in &m.fields {
        for a in &f.attrs {
            fields.push_str(&format!("    {a}\n"));
        }
        fields.push_str(&format!("    pub {}: {},\n", rust_ident(&f.name), f.ty));
    }
    format!(
        r#"//! Modelo de la colección `{coll}`. Generado con blacksmith junto con la
//! migración `{migration_module}`: si cambias uno, cambia el otro.

use ironforge::ODModel;
{bson_use}use serde::{{Deserialize, Serialize}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {struct_name} {{
{fields}}}

impl ODModel for {struct_name} {{
    fn coll_name() -> &'static str {{
        "{coll}"
    }}

    fn get_id(&self) -> Option<ObjectId> {{
        self.id
    }}

    fn set_id(&mut self, id: ObjectId) {{
        self.id = Some(id);
    }}
}}
"#
    )
}

/// Lo que hay que importar de `ironforge::migration` según el código.
fn migration_imports(code: &str, extra: &str) -> String {
    let mut names = vec!["Migration", "Schema", extra];
    for n in ["Item", "UniqueType", "IndexOptions"] {
        if code.contains(&format!("{n}::")) || code.contains(&format!("{n} {{")) {
            names.push(n);
        }
    }
    names.sort_unstable();
    names.join(", ")
}

#[cfg(test)]
mod tests {
    use super::super::spec;
    use super::*;

    #[test]
    fn migracion_y_modelo() {
        let lines =
            spec::parse("id()\nstring(\"email\").unique(UniqueType::Normal)\nint(\"type\").optional()").unwrap();
        let mig = migration_create("261125_1401_create_users", "CreateUsers", "users", &lines);
        assert!(mig.contains("use ironforge::migration::{Migration, Schema, UniqueType, Validation};"));
        assert!(mig.contains("            col.string(\"email\").unique(UniqueType::Normal);\n"));
        assert!(mig.contains("\"261125_1401_create_users\""));

        let model = model("User", "users", "m261125_1401_create_users", &spec::model(&lines));
        assert!(model.contains("use mongodb::bson::oid::ObjectId;\n"));
        assert!(model.contains(
            "    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub r#type: Option<i32>,"
        ));
        assert!(model.contains("impl ODModel for User"));
    }
}
