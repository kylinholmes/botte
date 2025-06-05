use axum::{
    http::StatusCode, Router,
};
use log::info;
use once_cell::sync::OnceCell;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable as ScalarServable};

use crate::{boardcast::BROADCAST_SENDER};

#[allow(dead_code)]
static API_TOKEN: OnceCell<String> = OnceCell::new();


pub fn api() -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(alert())
        .split_for_parts();

    if cfg!(debug_assertions) {
        info!("[debug mode] botte enable openapi with scalar");
        router.merge(Scalar::with_url("/", api))
    } else {
        router
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    tags(
        (name = "Botte", description = "Botte management",)
    )
)]
struct ApiDoc;
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("api_key"))),
            )
        }
    }
}


fn alert() -> OpenApiRouter {
    OpenApiRouter::new().nest(
        "/alert",
        OpenApiRouter::new()
            .routes(routes!(webhook))
            .routes(routes!(strategy)),
    )
}


#[utoipa::path(
    post,
    path = "/webhook",
    tags = ["alert"],
    request_body(
        content = String, 
        content_type = "text/plain",
        description = "alert msg"
    ),
    responses(
        (status = 200, description = "alert success")
    )
)]
async fn webhook(body: String) -> StatusCode {
    info!("[webhook] {}", body);
    if let Some(tx) = BROADCAST_SENDER.get() {
        if let Err(err) = tx.send(body).await {
            info!("Failed to send message: {}", err);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    StatusCode::OK
}


#[utoipa::path(
    post,
    path = "/strategy",
    tags = ["alert"],
    request_body(
        content = String, 
        content_type = "text/plain",
        description = "strategy info msg"
    ),
    responses(
        (status = 200, description = "strategy success")
    )
)]
async fn strategy(body: String) -> StatusCode {
    info!("[strategy] {}", body);
    StatusCode::OK
}