use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::any::{Any, TypeId};

use futures::lock::Mutex;
use std::collections::HashMap;

use actors_rs::Builder;

type RegistryAlias = HashMap<TypeId, Box<dyn Any + Send>>;
pub type Sender<T> = tokio::sync::mpsc::UnboundedSender<T>;

pub static REGISTRY: OnceCell<Mutex<RegistryAlias>> = OnceCell::new();

#[async_trait]
pub trait Registry: Builder
where
    Self: 'static,
{
    async fn register_once<T: Send + Sync + 'static>(sender: Sender<T>) {
        let registry = REGISTRY.get_or_init(Default::default);
        let mut registry = registry.lock().await;

        match registry.get_mut(&TypeId::of::<Self>()) {
            Some(_sender) => panic!("Actor already registered"),
            None => {
                registry.insert(TypeId::of::<Self>(), Box::new(sender));
                drop(registry);
            }
        }
    }

    async fn from_registry<T: Send + Sync + 'static>() -> Option<Sender<T>> {
        let registry = REGISTRY.get_or_init(Default::default);
        let mut registry = registry.lock().await;

        registry
            .get_mut(&TypeId::of::<Self>())
            .map(|addr| addr.downcast_ref::<Sender<T>>().unwrap().clone())
    }
}
