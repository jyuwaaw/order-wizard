use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, Bson, Document, Regex},
    error::{ErrorKind, WriteFailure},
    options::ReturnDocument,
    Collection,
};

use super::{
    ApplicationError, OrderSearch, TenantScopedOrderRepository, UpdateOrder, UpsertResult, UserId,
};
use crate::models::{Order, OrderEntity};

pub(crate) struct MongoOrderRepository {
    collection: Collection<OrderEntity>,
}

impl MongoOrderRepository {
    pub(crate) fn new(collection: Collection<OrderEntity>) -> Self {
        Self { collection }
    }
}

#[async_trait]
impl TenantScopedOrderRepository for MongoOrderRepository {
    async fn list(&self, user_id: &UserId) -> Result<Vec<Order>, ApplicationError> {
        let entities: Vec<OrderEntity> = self
            .collection
            .find(doc! { "user_id": user_id.as_str() })
            .await
            .map_err(repository_error)?
            .try_collect()
            .await
            .map_err(repository_error)?;

        Ok(entities.into_iter().map(Order::from).collect())
    }

    async fn get(
        &self,
        user_id: &UserId,
        order_id: &str,
    ) -> Result<Option<Order>, ApplicationError> {
        self.collection
            .find_one(doc! { "user_id": user_id.as_str(), "id": order_id })
            .await
            .map(|entity| entity.map(Order::from))
            .map_err(repository_error)
    }

    async fn search(
        &self,
        user_id: &UserId,
        search: OrderSearch,
    ) -> Result<Vec<Order>, ApplicationError> {
        let mut filter = doc! { "user_id": user_id.as_str() };
        if let Some(status) = search.status {
            filter.insert(
                "status",
                mongodb::bson::to_bson(&status)
                    .map_err(|error| ApplicationError::Repository(error.to_string()))?,
            );
        }
        if let Some(query) = search
            .query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            let regex = Regex {
                pattern: regex::escape(query),
                options: "i".to_string(),
            };
            filter.insert(
                "$or",
                vec![
                    doc! { "id": regex.clone() },
                    doc! { "order_number": regex.clone() },
                    doc! { "product_name": regex.clone() },
                    doc! { "note": regex },
                ],
            );
        }

        let entities: Vec<OrderEntity> = self
            .collection
            .find(filter)
            .limit(search.limit as i64)
            .await
            .map_err(repository_error)?
            .try_collect()
            .await
            .map_err(repository_error)?;
        Ok(entities.into_iter().map(Order::from).collect())
    }

    async fn upsert_if_newer(
        &self,
        user_id: &UserId,
        order: Order,
    ) -> Result<UpsertResult, ApplicationError> {
        let incoming_timestamp = order
            .updated_at
            .as_deref()
            .or(order.created_at.as_deref())
            .unwrap_or("");
        let comparison = if incoming_timestamp.is_empty() {
            "$lte"
        } else {
            "$lt"
        };
        let mut timestamp_comparison = Document::new();
        timestamp_comparison.insert(
            comparison,
            Bson::Array(vec![
                Bson::Document(doc! {
                    "$ifNull": ["$updated_at", { "$ifNull": ["$created_at", ""] }]
                }),
                Bson::String(incoming_timestamp.to_string()),
            ]),
        );
        let filter = doc! {
            "user_id": user_id.as_str(),
            "order_number": &order.order_number,
            "$expr": timestamp_comparison,
        };

        match self
            .collection
            .replace_one(filter, OrderEntity::from(order.clone()))
            .upsert(true)
            .await
        {
            Ok(result) => Ok(UpsertResult {
                order,
                applied: result.modified_count > 0 || result.upserted_id.is_some(),
            }),
            Err(error) if is_duplicate_key(&error) => self
                .collection
                .find_one(doc! {
                    "user_id": user_id.as_str(),
                    "order_number": &order.order_number,
                })
                .await
                .map_err(repository_error)?
                .map(|entity| UpsertResult {
                    order: Order::from(entity),
                    applied: false,
                })
                .ok_or_else(|| {
                    ApplicationError::Repository(
                        "conditional upsert conflicted but canonical order was missing".to_string(),
                    )
                }),
            Err(error) => Err(repository_error(error)),
        }
    }

    async fn update_status(
        &self,
        user_id: &UserId,
        order_id: &str,
        status: crate::models::OrderStatus,
        updated_at: &str,
    ) -> Result<Option<Order>, ApplicationError> {
        self.collection
            .find_one_and_update(
                doc! { "user_id": user_id.as_str(), "id": order_id },
                doc! {
                    "$set": {
                        "status": mongodb::bson::to_bson(&status).map_err(|error| {
                            ApplicationError::Repository(error.to_string())
                        })?,
                        "updated_at": updated_at,
                    }
                },
            )
            .return_document(ReturnDocument::After)
            .await
            .map(|entity| entity.map(Order::from))
            .map_err(repository_error)
    }

    async fn update_note(
        &self,
        user_id: &UserId,
        order_id: &str,
        note: &str,
        updated_at: &str,
    ) -> Result<Option<Order>, ApplicationError> {
        self.collection
            .find_one_and_update(
                doc! { "user_id": user_id.as_str(), "id": order_id },
                doc! {
                    "$set": {
                        "note": note,
                        "updated_at": updated_at,
                    }
                },
            )
            .return_document(ReturnDocument::After)
            .await
            .map(|entity| entity.map(Order::from))
            .map_err(repository_error)
    }

    async fn delete(&self, user_id: &UserId, order_id: &str) -> Result<bool, ApplicationError> {
        self.collection
            .delete_one(doc! { "user_id": user_id.as_str(), "id": order_id })
            .await
            .map(|result| result.deleted_count > 0)
            .map_err(repository_error)
    }

    async fn update(
        &self,
        user_id: &UserId,
        order_id: &str,
        update: UpdateOrder,
    ) -> Result<Option<Order>, ApplicationError> {
        let mut fields = Document::new();
        if let Some(status) = update.status {
            fields.insert(
                "status",
                mongodb::bson::to_bson(&status)
                    .map_err(|error| ApplicationError::Repository(error.to_string()))?,
            );
        }
        if let Some(note) = update.note {
            fields.insert("note", note);
        }
        if let Some(updated_at) = update.updated_at {
            fields.insert("updated_at", updated_at);
        }
        if let Some(deleted_at) = update.deleted_at {
            fields.insert("deleted_at", deleted_at);
        }

        self.collection
            .find_one_and_update(
                doc! { "user_id": user_id.as_str(), "id": order_id },
                doc! { "$set": fields },
            )
            .return_document(ReturnDocument::After)
            .await
            .map(|entity| entity.map(Order::from))
            .map_err(repository_error)
    }

    async fn delete_many(
        &self,
        user_id: &UserId,
        order_ids: &[String],
    ) -> Result<usize, ApplicationError> {
        self.collection
            .delete_many(doc! {
                "user_id": user_id.as_str(),
                "id": { "$in": order_ids },
            })
            .await
            .map(|result| result.deleted_count as usize)
            .map_err(repository_error)
    }
}

fn is_duplicate_key(error: &mongodb::error::Error) -> bool {
    matches!(
        error.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(write_error)) if write_error.code == 11000
    )
}

fn repository_error(error: mongodb::error::Error) -> ApplicationError {
    ApplicationError::Repository(error.to_string())
}
