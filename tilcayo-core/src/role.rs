use std::collections::HashMap;

use crate::{Permission, SchemaId};

pub struct RoleId(pub usize);

pub struct Role {
    pub id: RoleId,
    pub permissions: HashMap<SchemaId, Permission>,
}

impl Role {
    pub fn can_read(&self, schema_id: &SchemaId) -> bool {
        self.permissions
            .get(schema_id)
            .is_some_and(|permission| permission.read)
    }

    pub fn can_write(&self, schema_id: &SchemaId) -> bool {
        self.permissions
            .get(schema_id)
            .is_some_and(|permission| permission.write)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{Permission, Role, RoleId, SchemaId};

    #[test]
    fn role_can_read() {
        let schema_id = SchemaId(1);

        let role = Role {
            id: RoleId(1),
            permissions: HashMap::from([(
                schema_id,
                Permission {
                    read: true,
                    write: false,
                },
            )]),
        };

        assert!(role.can_read(&schema_id));
        assert!(!role.can_write(&schema_id));
    }

    #[test]
    fn role_cannot_access_schema_without_permission() {
        let role = Role {
            id: RoleId(1),
            permissions: HashMap::new(),
        };

        assert!(!role.can_read(&SchemaId(1)));
        assert!(!role.can_write(&SchemaId(1)));
    }
}
