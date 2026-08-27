use std::sync::RwLock;

use async_trait::async_trait;

use super::{
    ApplicationError, OrderSearch, TenantScopedOrderRepository, UpdateOrder, UpsertResult, UserId,
};
use crate::models::{Order, OrderStatus};

pub(crate) struct InMemoryOrderRepository {
    orders: RwLock<Vec<Order>>,
}

impl InMemoryOrderRepository {
    pub(crate) fn with_orders(orders: impl IntoIterator<Item = Order>) -> Self {
        Self {
            orders: RwLock::new(orders.into_iter().collect()),
        }
    }
}

#[async_trait]
impl TenantScopedOrderRepository for InMemoryOrderRepository {
    async fn list(&self, user_id: &UserId) -> Result<Vec<Order>, ApplicationError> {
        Ok(self
            .orders
            .read()
            .expect("in-memory order repository lock poisoned")
            .iter()
            .filter(|order| order.user_id == user_id.as_str())
            .cloned()
            .collect())
    }

    async fn get(
        &self,
        user_id: &UserId,
        order_id: &str,
    ) -> Result<Option<Order>, ApplicationError> {
        Ok(self
            .orders
            .read()
            .expect("in-memory order repository lock poisoned")
            .iter()
            .find(|order| order.user_id == user_id.as_str() && order.id == order_id)
            .cloned())
    }

    async fn search(
        &self,
        user_id: &UserId,
        search: OrderSearch,
    ) -> Result<Vec<Order>, ApplicationError> {
        let query = search
            .query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .map(str::to_lowercase);
        let orders = self
            .orders
            .read()
            .expect("in-memory order repository lock poisoned");
        Ok(orders
            .iter()
            .filter(|order| order.user_id == user_id.as_str())
            .filter(|order| {
                search
                    .status
                    .as_ref()
                    .is_none_or(|status| &order.status == status)
            })
            .filter(|order| {
                query.as_ref().is_none_or(|query| {
                    order.id.to_lowercase().contains(query)
                        || order.order_number.to_lowercase().contains(query)
                        || order.product_name.to_lowercase().contains(query)
                        || order
                            .note
                            .as_deref()
                            .is_some_and(|note| note.to_lowercase().contains(query))
                })
            })
            .take(search.limit)
            .cloned()
            .collect())
    }

    async fn upsert_if_newer(
        &self,
        user_id: &UserId,
        order: Order,
    ) -> Result<UpsertResult, ApplicationError> {
        let mut orders = self
            .orders
            .write()
            .expect("in-memory order repository lock poisoned");
        if let Some(existing) = orders.iter_mut().find(|existing| {
            existing.user_id == user_id.as_str() && existing.order_number == order.order_number
        }) {
            if should_replace(existing, &order) {
                *existing = order.clone();
                Ok(UpsertResult {
                    order,
                    applied: true,
                })
            } else {
                Ok(UpsertResult {
                    order: existing.clone(),
                    applied: false,
                })
            }
        } else {
            orders.push(order.clone());
            Ok(UpsertResult {
                order,
                applied: true,
            })
        }
    }

    async fn update_status(
        &self,
        user_id: &UserId,
        order_id: &str,
        status: OrderStatus,
        updated_at: &str,
    ) -> Result<Option<Order>, ApplicationError> {
        let mut orders = self
            .orders
            .write()
            .expect("in-memory order repository lock poisoned");
        let order = orders
            .iter_mut()
            .find(|order| order.user_id == user_id.as_str() && order.id == order_id);
        if let Some(order) = order {
            order.status = status;
            order.updated_at = Some(updated_at.to_string());
            Ok(Some(order.clone()))
        } else {
            Ok(None)
        }
    }

    async fn update_note(
        &self,
        user_id: &UserId,
        order_id: &str,
        note: &str,
        updated_at: &str,
    ) -> Result<Option<Order>, ApplicationError> {
        let mut orders = self
            .orders
            .write()
            .expect("in-memory order repository lock poisoned");
        let order = orders
            .iter_mut()
            .find(|order| order.user_id == user_id.as_str() && order.id == order_id);
        if let Some(order) = order {
            order.note = Some(note.to_string());
            order.updated_at = Some(updated_at.to_string());
            Ok(Some(order.clone()))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, user_id: &UserId, order_id: &str) -> Result<bool, ApplicationError> {
        let mut orders = self
            .orders
            .write()
            .expect("in-memory order repository lock poisoned");
        let original_len = orders.len();
        orders.retain(|order| !(order.user_id == user_id.as_str() && order.id == order_id));
        Ok(orders.len() != original_len)
    }

    async fn update(
        &self,
        user_id: &UserId,
        order_id: &str,
        update: UpdateOrder,
    ) -> Result<Option<Order>, ApplicationError> {
        let mut orders = self
            .orders
            .write()
            .expect("in-memory order repository lock poisoned");
        let order = orders
            .iter_mut()
            .find(|order| order.user_id == user_id.as_str() && order.id == order_id);
        if let Some(order) = order {
            if let Some(status) = update.status {
                order.status = status;
            }
            if let Some(note) = update.note {
                order.note = Some(note);
            }
            if let Some(updated_at) = update.updated_at {
                order.updated_at = Some(updated_at);
            }
            if let Some(deleted_at) = update.deleted_at {
                order.deleted_at = Some(deleted_at);
            }
            Ok(Some(order.clone()))
        } else {
            Ok(None)
        }
    }

    async fn delete_many(
        &self,
        user_id: &UserId,
        order_ids: &[String],
    ) -> Result<usize, ApplicationError> {
        let mut orders = self
            .orders
            .write()
            .expect("in-memory order repository lock poisoned");
        let original_len = orders.len();
        orders.retain(|order| {
            order.user_id != user_id.as_str() || !order_ids.iter().any(|id| id == &order.id)
        });
        Ok(original_len - orders.len())
    }
}

fn should_replace(existing: &Order, incoming: &Order) -> bool {
    match (effective_timestamp(existing), effective_timestamp(incoming)) {
        (Some(existing), Some(incoming)) => incoming > existing,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (None, None) => true,
    }
}

fn effective_timestamp(order: &Order) -> Option<&str> {
    order.updated_at.as_deref().or(order.created_at.as_deref())
}
