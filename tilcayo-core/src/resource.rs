use crate::{SchemaId, Value};

pub struct ResourceId(pub usize);

pub struct Resource {
    pub id: ResourceId,
    pub schema_id: SchemaId,
    pub data: Value,
}
