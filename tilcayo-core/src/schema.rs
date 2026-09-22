use crate::ValueType;

pub struct SchemaId(pub usize);

pub struct Schema {
    pub id: SchemaId,
    pub schema: ValueType,
}
