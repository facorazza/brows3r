use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;
use uuid::Uuid;

use crate::{error::AppError, models::User, state::AppState};

const USER_ID_KEY: &str = "user_id";

pub async fn require_auth<DB: sqlx::Database>(
    State(state): State<AppState<DB>>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Result<Response, Response>
where
    for<'c> &'c mut DB::Connection: sqlx::Executor<'c, Database = DB>,
    for<'q> <DB as sqlx::Database>::Arguments<'q>: sqlx::IntoArguments<'q, DB>,
    for<'r> User: sqlx::FromRow<'r, DB::Row>,
    for<'q> Uuid: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    let user_id: Option<Uuid> = session.get(USER_ID_KEY).await.ok().flatten();

    if let Some(user_id) = user_id {
        // Verify user still exists in database
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking user {}: {:?}", user_id, e);
                Redirect::to("/users/login").into_response()
            })?;

        if let Some(user) = user {
            request.extensions_mut().insert(user);
            return Ok(next.run(request).await);
        }
    }

    // Not authenticated, redirect to login
    Ok(Redirect::to(&format!("/users/login?next={}", request.uri().path())).into_response())
}

pub async fn authenticate_user<DB: sqlx::Database>(
    pool: &sqlx::Pool<DB>,
    username: &str,
    password: &str,
    #[cfg(feature = "ldap")] config: &crate::config::Config,
) -> Result<User, AppError>
where
    for<'c> &'c mut DB::Connection: sqlx::Executor<'c, Database = DB>,
    for<'q> <DB as sqlx::Database>::Arguments<'q>: sqlx::IntoArguments<'q, DB>,
    for<'r> User: sqlx::FromRow<'r, DB::Row>,
    for<'q> &'q str: sqlx::Encode<'q, DB>,
    for<'q> String: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
    str: sqlx::Type<DB>,
{
    // First, try local authentication
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await?;

    if let Some(user) = user
        && user.verify_password(password)
    {
        return Ok(user);
    }

    // If local auth fails and LDAP is enabled, try LDAP
    #[cfg(feature = "ldap")]
    {
        if config.ldap_server_uri.is_some() {
            return authenticate_ldap(pool, username, password, config).await;
        }
    }

    Err(AppError::Authentication)
}

#[cfg(feature = "ldap")]
async fn authenticate_ldap<DB: sqlx::Database>(
    pool: &sqlx::Pool<DB>,
    username: &str,
    password: &str,
    config: &crate::config::Config,
) -> Result<User, AppError>
where
    for<'c> &'c mut DB::Connection: sqlx::Executor<'c, Database = DB>,
    for<'q> <DB as sqlx::Database>::Arguments<'q>: sqlx::IntoArguments<'q, DB>,
    for<'r> User: sqlx::FromRow<'r, DB::Row>,
    for<'q> &'q str: sqlx::Encode<'q, DB>,
    for<'q> String: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
    str: sqlx::Type<DB>,
{
    use ldap3::LdapConnAsync;

    let ldap_uri = config
        .ldap_server_uri
        .as_ref()
        .ok_or(AppError::Authentication)?;

    let (conn, mut ldap) = LdapConnAsync::new(ldap_uri).await?;
    ldap3::drive!(conn);

    // Bind with service account if provided
    if let (Some(bind_dn), Some(bind_password)) = (&config.ldap_bind_dn, &config.ldap_bind_password)
    {
        ldap.simple_bind(bind_dn, bind_password).await?;
    }

    // Search for user
    let user_dn_template = config
        .ldap_user_dn_template
        .as_ref()
        .ok_or(AppError::Authentication)?;

    let user_dn = user_dn_template.replace("%(user)s", username);

    // Try to bind as the user
    ldap.simple_bind(&user_dn, password).await?;

    // If we get here, authentication was successful
    // Check if user exists in local database, create if not
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await?;

    if let Some(user) = user {
        Ok(user)
    } else {
        // Create new user from LDAP
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email) VALUES ($1, $2) RETURNING *",
        )
        .bind(username)
        .bind(format!("{}@example.com", username)) // You might want to fetch this from LDAP
        .fetch_one(pool)
        .await?;

        Ok(user)
    }
}

pub async fn login_user(session: &Session, user_id: Uuid) -> Result<(), AppError> {
    session
        .insert(USER_ID_KEY, user_id)
        .await
        .map_err(|_| AppError::Internal("Failed to create session".to_string()))?;
    // Rotate the session id on privilege change to prevent fixation
    session
        .cycle_id()
        .await
        .map_err(|_| AppError::Internal("Failed to rotate session id".to_string()))?;
    Ok(())
}

pub async fn logout_user(session: &Session) -> Result<(), AppError> {
    session
        .delete()
        .await
        .map_err(|_| AppError::Internal("Failed to delete session".to_string()))?;
    Ok(())
}
