use crate::{Schema, SchemaId, Value};

pub struct ResourceId(pub usize);

pub struct Resource {
    pub id: ResourceId,
    pub schema_id: SchemaId,
    pub data: Value,
}

impl Resource {
    pub fn validate(&self, schema: &Schema) -> bool {
        schema.schema.matches(&self.data)
    }
}
