use actix_web::cookie::Cookie;
use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

// #[derive(serde::Deserialize)]
// pub struct QueryParams {
//     error: String,
//     tag: String,
// }

// impl QueryParams {
// fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
//     let tag = hex::decode(self.tag)?;
//     let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));
//     let mut mac =
//         Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
//     mac.update(query_string.as_bytes());
//     mac.verify_slice(&tag)?;
//     Ok(self.error)
// }
// }

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    // Start - v1 HmacSecret
    // param query: Option<web::Query<QueryParams>>,secret: web::Data<HmacSecret>
    // let error_html = match query {
    //     None => "".into(),
    //     Some(query) => match query.0.verify(&secret) {
    //         Ok(error) => {
    //             format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error))
    //         }
    //         Err(e) => {
    //             tracing::warn!(
    //             error.message = %e,
    //             error.cause_chain = ?e,
    //             "Failed to verify query parameters using the HMAC tag"
    //             );
    //             "".into()
    //         }
    //     },
    // };
    // End - v1 HmacSecret

    // Start - v2 set cookie
    // let error_html = match request.cookie("_flash") {
    //     None => "".into(),
    //     Some(cookie) => {
    //         format!("<p><i>{}</i></p>", cookie.value())
    //     }
    // };
    // send - .cookie(Cookie::new("_flash", e.to_string()))
    // receive - .cookie(Cookie::build("_flash", "").max_age(Duration::ZERO).finish())
    // End - v2 set cookie

    let mut error_html = String::new();

    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let mut response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/html; charset=utf-8">
                    <title>Login</title>
                </head>
                <body>
                    {error_html}
                    <form action="/login" method="post">
                        <label>Username
                            <input
                                type="text"
                                placeholder="Enter Username"
                                name="username"
                > </label>
                        <label>Password
                            <input
                                type="password"
                                placeholder="Enter Password"
                                name="password"
                > </label>
                        <button type="submit">Login</button>
                    </form>
                </body>
                </html>"#,
        ));

    response
        .add_removal_cookie(&Cookie::new("_flash", ""))
        .unwrap();
    response
}
