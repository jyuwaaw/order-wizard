use mongodb::{bson::doc, Client, Collection, Database};

use crate::models::OrderEntity;

pub async fn connect() -> Result<Database, mongodb::error::Error> {
    let uri =
        std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
    let client = Client::with_uri_str(&uri).await?;
    let db = client.database("order_wizard");

    // Ping to verify connection
    db.run_command(doc! { "ping": 1 }).await?;
    tracing::info!("Connected to MongoDB");

    Ok(db)
}

pub fn orders_collection(database: &Database) -> Collection<OrderEntity> {
    database.collection("orders")
}
