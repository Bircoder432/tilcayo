use crate::RoleId;

#[derive(Clone, Copy)]
pub struct UserId(pub usize);

#[derive(Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub role_id: RoleId,
    pub password_hash: String,
}
