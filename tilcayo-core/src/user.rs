use crate::RoleId;

#[derive(Clone, Copy)]
pub struct UserId(pub usize);

pub struct User {
    pub id: UserId,
    pub role_id: RoleId,
}
