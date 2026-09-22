use std::collections::HashMap;

use crate::{Permission, SchemaId};

pub struct RoleId(pub usize);

pub struct Role {
    pub id: RoleId,
    pub permissions: HashMap<SchemaId, Permission>,
}
