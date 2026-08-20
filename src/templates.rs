use crate::models::S3Object;
use askama::Template;

#[derive(Template)]
#[template(path = "browser/list.html")]
pub struct BrowserListTemplate {
    pub path: String,
    pub files: Vec<S3Object>,
    pub directories: Vec<S3Object>,
}

#[derive(Template)]
#[template(path = "users/login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
    pub next: Option<String>,
}

#[derive(Template)]
#[template(path = "users/list.html")]
pub struct UserListTemplate {
    pub users: Vec<crate::models::User>,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub status_code: u16,
    pub title: String,
    pub message: String,
}
