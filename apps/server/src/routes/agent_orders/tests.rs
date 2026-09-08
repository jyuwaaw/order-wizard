use super::router;
use crate::application::{InMemoryOrderRepository, OrderApplication, Principal, UserId};
use crate::models::{Order, OrderStatus};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension,
};
use tower::ServiceExt;

fn order(id: &str, user_id: &str, product_name: &str) -> Order {
    Order {
        id: id.to_string(),
        user_id: user_id.to_string(),
        order_number: format!("{id}-order-number"),
        product_name: product_name.to_string(),
        order_date: "August 26, 2026".to_string(),
        product_image: "https://example.com/product.jpg".to_string(),
        price: "$10.00".to_string(),
        status: OrderStatus::Uncommented,
        note: None,
        updated_at: Some("2026-08-26T12:00:00Z".to_string()),
        created_at: Some("2026-08-26T12:00:00Z".to_string()),
        deleted_at: None,
    }
}

fn agent_principal(user_id: &str) -> Principal {
    Principal::agent(UserId::new(user_id))
}

#[tokio::test]
async fn search_returns_only_matching_orders_for_the_authenticated_user() {
    let application = OrderApplication::new(InMemoryOrderRepository::with_orders([
        order("alice-match", "alice", "Wireless Headphones"),
        order("alice-miss", "alice", "Keyboard"),
        order("bob-match", "bob", "Wireless Headphones"),
    ]));
    let app: axum::Router = router()
        .layer(Extension(application))
        .layer(Extension(agent_principal("alice")))
        .into();

    let response = app
        .oneshot(
            Request::get("/agent/orders?q=headphones&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["id"], "alice-match");
}

#[tokio::test]
async fn status_update_returns_the_canonical_updated_order() {
    let application = OrderApplication::new(InMemoryOrderRepository::with_orders([order(
        "alice-order",
        "alice",
        "Wireless Headphones",
    )]));
    let app: axum::Router = router()
        .layer(Extension(application))
        .layer(Extension(agent_principal("alice")))
        .into();

    let response = app
        .oneshot(
            Request::patch("/agent/orders/alice-order/status")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"status":"reimbursed"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], "alice-order");
    assert_eq!(json["status"], "reimbursed");
    assert!(json["updatedAt"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

#[tokio::test]
async fn note_update_returns_the_canonical_updated_order() {
    let application = OrderApplication::new(InMemoryOrderRepository::with_orders([order(
        "alice-order",
        "alice",
        "Wireless Headphones",
    )]));
    let app: axum::Router = router()
        .layer(Extension(application))
        .layer(Extension(agent_principal("alice")))
        .into();

    let response = app
        .oneshot(
            Request::patch("/agent/orders/alice-order/note")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"note":"Follow up tomorrow"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["note"], "Follow up tomorrow");
    assert!(json["updatedAt"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

#[tokio::test]
async fn detail_returns_only_the_authenticated_users_order() {
    let application = OrderApplication::new(InMemoryOrderRepository::with_orders([
        order("shared-id", "alice", "Alice product"),
        order("shared-id", "bob", "Bob product"),
    ]));
    let app: axum::Router = router()
        .layer(Extension(application))
        .layer(Extension(agent_principal("alice")))
        .into();

    let response = app
        .oneshot(
            Request::get("/agent/orders/shared-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["productName"], "Alice product");
}

#[tokio::test]
async fn agent_router_has_no_create_or_delete_operations() {
    let application = OrderApplication::new(InMemoryOrderRepository::with_orders([order(
        "alice-order",
        "alice",
        "Alice product",
    )]));
    let app: axum::Router = router()
        .layer(Extension(application))
        .layer(Extension(agent_principal("alice")))
        .into();

    let create = app
        .clone()
        .oneshot(
            Request::post("/agent/orders")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let delete = app
        .oneshot(
            Request::delete("/agent/orders/alice-order")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(delete.status(), StatusCode::METHOD_NOT_ALLOWED);
}
