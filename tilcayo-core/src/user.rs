use crate::RoleId;

pub struct UserId(pub usize);

pub struct User {
    pub id: UserId,
    pub role_id: RoleId,
}
