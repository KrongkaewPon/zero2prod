use actix_web::web::ReqData;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::idempotency::get_saved_response;
use crate::idempotency::save_response;
use crate::idempotency::IdempotencyKey;
use crate::idempotency::{try_processing, NextAction};
use crate::utils::{e400, e500, see_other};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

// #[derive(thiserror::Error)]
// pub enum PublishError {
//     #[error("Authentication failed.")]
//     AuthError(#[source] anyhow::Error),
//     #[error(transparent)]
//     UnexpectedError(#[from] anyhow::Error),
// }

// impl std::fmt::Debug for PublishError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         error_chain_fmt(self, f)
//     }
// }

// use actix_web::http::header::HeaderValue;
// use actix_web::http::{header, StatusCode};
// impl ResponseError for PublishError {
//     fn error_response(&self) -> HttpResponse {
//         match self {
//             PublishError::UnexpectedError(_) => {
//                 HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
//             }
//             PublishError::AuthError(_) => {
//                 let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
//                 let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
//                 response
//                     .headers_mut()
//                     // actix_web::http::header provides a collection of constants // for the names of several well-known/standard HTTP headers
//                     .insert(header::WWW_AUTHENTICATE, header_value);
//                 response
//             }
//         }
//     }

//     fn status_code(&self) -> StatusCode {
//         match self {
//             PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             PublishError::AuthError(_) => StatusCode::UNAUTHORIZED,
//         }
//     }
// }

#[tracing::instrument(
name = "Publish a newsletter issue", skip_all, fields(user_id=%&*user_id)
)]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    user_id: ReqData<UserId>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    // We must destructure the form to avoid upsetting the borrow-checker
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("Failed to store newsletter issue details")
        .map_err(e500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    // let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    // for subscriber in subscribers {
    //     match subscriber {
    //         Ok(subscriber) => {
    //             email_client
    //                 .send_email(&subscriber.email, &title, &html_content, &text_content)
    //                 .await
    //                 .with_context(|| {
    //                     format!("Failed to send newsletter issue to {}", subscriber.email)
    //                 })
    //                 .map_err(e500)?;
    //         }
    //         Err(error) => {
    //             tracing::warn!(
    //             // We record the error chain as a structured field // on the log record.
    //             error.cause_chain = ?error,
    //             error.message = %error,
    //             // Using `\` to split a long string literal over // two lines, without creating a `\n` character.
    //             "Skipping a confirmed subscriber. \
    //             Their stored contact details are invalid",
    //             );
    //         }
    //     }
    // }

    // success_message().send();

    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;

    success_message().send();
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info(
        "The newsletter issue has been accepted - \
        emails will go out shortly.",
    )
}

// struct ConfirmedSubscriber {
//     email: SubscriberEmail,
// }
// #[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
// async fn get_confirmed_subscribers(
//     pool: &PgPool,
// ) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
//     // We only need `Row` to map the data coming out of this query.
//     // Nesting its definition inside the function itself is a simple way
//     // to clearly communicate this coupling (and to ensure it doesn't
//     // get used elsewhere by mistake).
//     struct Row {
//         email: String,
//     }

//     let rows = sqlx::query_as!(
//         Row,
//         r#"
//         SELECT email
//         FROM subscriptions
//         WHERE status = 'confirmed'
//         "#,
//     )
//     .fetch_all(pool)
//     .await?;

//     let confirmed_subscribers = rows
//         .into_iter()
//         .map(|r| match SubscriberEmail::parse(r.email) {
//             Ok(email) => Ok(ConfirmedSubscriber { email }),
//             Err(error) => Err(anyhow::anyhow!(error)),
//         })
//         .collect();

//     Ok(confirmed_subscribers)
// }

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    )
    .execute(transaction)
    .await?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(transaction)
    .await?;
    Ok(())
}
