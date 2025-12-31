use axum::{
    Extension, Form,
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth::{authenticate_user, login_user, logout_user},
    error::AppError,
    models::User,
    state::AppState,
    templates::{LoginTemplate, UserListTemplate},
};

pub async fn user_list(
    State(state): State<AppState>,
    Extension(_user): Extension<User>,
) -> Result<Html<String>, AppError> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY username")
        .fetch_all(&state.pool)
        .await?;

    let template = UserListTemplate { users };

    let html = template
        .render()
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct LoginQuery {
    next: Option<String>,
}

pub async fn login_form(Query(query): Query<LoginQuery>) -> Result<Html<String>, AppError> {
    let template = LoginTemplate {
        error: None,
        next: query.next,
    };

    let html = template
        .render()
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<LoginQuery>,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, AppError> {
    let user = authenticate_user(
        &state.pool,
        &form.username,
        &form.password,
        #[cfg(feature = "ldap")]
        &state.config,
    )
    .await;

    match user {
        Ok(user) => {
            tracing::info!("User {} logged in successfully", user.username);
            login_user(&session, user.id).await?;
            let redirect_to = query.next.unwrap_or_else(|| "/".to_string());
            Ok(Redirect::to(&redirect_to).into_response())
        }
        Err(_) => {
            tracing::warn!("Failed login attempt for username: {}", form.username);
            let template = LoginTemplate {
                error: Some("Invalid username or password".to_string()),
                next: query.next,
            };
            let html = template
                .render()
                .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;
            Ok(Html(html).into_response())
        }
    }
}

pub async fn logout(session: Session) -> Result<Redirect, AppError> {
    logout_user(&session).await?;
    Ok(Redirect::to("/users/login/"))
}
