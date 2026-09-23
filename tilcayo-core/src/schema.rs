use crate::ValueType;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct SchemaId(pub usize);

pub struct Schema {
    pub id: SchemaId,
    pub name: String,
    pub schema: ValueType,
}
