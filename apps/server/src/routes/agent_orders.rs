use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::application::{OrderApplication, OrderSearch};
use crate::auth::{AuthAgentPrincipal, AuthError};
use crate::errors::AppResult;
use crate::models::{Order, OrderStatus};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct AgentOrderQuery {
    /// Case-insensitive text matched against ID, order number, product name, and note.
    q: Option<String>,
    /// Optional exact order status.
    status: Option<OrderStatus>,
    /// Maximum results from 1 through 100. Defaults to 50.
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct AgentStatusUpdateRequest {
    status: OrderStatus,
}

#[derive(Debug, Deserialize, ToSchema)]
struct AgentNoteUpdateRequest {
    note: String,
}

pub fn router() -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(search_orders))
        .routes(routes!(get_order))
        .routes(routes!(update_status))
        .routes(routes!(update_note))
}

#[utoipa::path(
    get,
    path = "/agent/orders",
    tag = "Agent Orders",
    summary = "Search the authenticated user's orders",
    params(AgentOrderQuery),
    responses(
        (status = 200, description = "Matching orders", body = Vec<Order>),
        (status = 400, description = "Invalid search parameters"),
        (status = 401, description = "Unauthorized", body = AuthError)
    ),
    security(("bearer_auth" = []))
)]
async fn search_orders(
    Extension(application): Extension<OrderApplication>,
    AuthAgentPrincipal(principal): AuthAgentPrincipal,
    Query(query): Query<AgentOrderQuery>,
) -> AppResult<Json<Vec<Order>>> {
    let orders = application
        .search_orders(
            &principal,
            OrderSearch {
                query: query.q,
                status: query.status,
                limit: query.limit.unwrap_or(50),
            },
        )
        .await?;
    Ok(Json(orders))
}

#[utoipa::path(
    get,
    path = "/agent/orders/{id}",
    tag = "Agent Orders",
    summary = "Get one order",
    params(("id" = String, Path, description = "Order ID")),
    responses(
        (status = 200, description = "Order detail", body = Order),
        (status = 404, description = "Order not found"),
        (status = 401, description = "Unauthorized", body = AuthError)
    ),
    security(("bearer_auth" = []))
)]
async fn get_order(
    Extension(application): Extension<OrderApplication>,
    AuthAgentPrincipal(principal): AuthAgentPrincipal,
    Path(id): Path<String>,
) -> AppResult<Json<Order>> {
    let order = application.get_order(&principal, &id).await?;
    Ok(Json(order))
}

#[utoipa::path(
    patch,
    path = "/agent/orders/{id}/status",
    tag = "Agent Orders",
    summary = "Update an order's status",
    params(("id" = String, Path, description = "Order ID")),
    request_body = AgentStatusUpdateRequest,
    responses(
        (status = 200, description = "Canonical updated order", body = Order),
        (status = 404, description = "Order not found"),
        (status = 401, description = "Unauthorized", body = AuthError)
    ),
    security(("bearer_auth" = []))
)]
async fn update_status(
    Extension(application): Extension<OrderApplication>,
    AuthAgentPrincipal(principal): AuthAgentPrincipal,
    Path(id): Path<String>,
    Json(request): Json<AgentStatusUpdateRequest>,
) -> AppResult<Json<Order>> {
    let order = application
        .update_status(&principal, &id, request.status)
        .await?;
    Ok(Json(order))
}

#[utoipa::path(
    patch,
    path = "/agent/orders/{id}/note",
    tag = "Agent Orders",
    summary = "Update an order's note",
    params(("id" = String, Path, description = "Order ID")),
    request_body = AgentNoteUpdateRequest,
    responses(
        (status = 200, description = "Canonical updated order", body = Order),
        (status = 404, description = "Order not found"),
        (status = 401, description = "Unauthorized", body = AuthError)
    ),
    security(("bearer_auth" = []))
)]
async fn update_note(
    Extension(application): Extension<OrderApplication>,
    AuthAgentPrincipal(principal): AuthAgentPrincipal,
    Path(id): Path<String>,
    Json(request): Json<AgentNoteUpdateRequest>,
) -> AppResult<Json<Order>> {
    let order = application
        .update_note(&principal, &id, request.note)
        .await?;
    Ok(Json(order))
}

#[cfg(test)]
mod tests;
