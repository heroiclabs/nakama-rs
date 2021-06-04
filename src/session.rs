#[derive(Debug)]
pub struct Session {
    pub auth_token: String,
    pub refresh_token: Option<String>,
}
