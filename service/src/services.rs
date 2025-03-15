use async_trait::async_trait;
use error_stack::Result;
use lib::error::Error;
use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::debug;

/// Manage shared services across the application
#[derive(Clone)]
pub struct ServiceProvider {
    inner: Arc<ServicesInner>,
}

/// Hold cache of the initialized services
struct ServicesInner {
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl ServiceProvider {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        ServiceProvider {
            inner: Arc::new(ServicesInner {
                services: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Check if the service is already initialized and stored in cache
    pub async fn has_service<T>(&self) -> bool
    where
        T: ServiceFactory + Send + Sync + 'static,
    {
        self.inner
            .services
            .read()
            .await
            .contains_key(&TypeId::of::<T>())
    }

    /// Add a new service to the cache
    ///
    /// Note: This assume that service already initialized
    pub async fn add_service<T>(&self, service: T) -> Arc<T>
    where
        T: ServiceFactory + Send + Sync + 'static,
    {
        debug!(name = type_name::<T>(), "Caching service");

        let type_id = TypeId::of::<T>();
        let service = Arc::new(service);

        self.inner
            .services
            .write()
            .await
            .insert(type_id, service.clone());

        service
    }

    /// Remove the service from cache
    pub async fn remove_service<T>(&self)
    where
        T: ServiceFactory + Send + Sync + 'static,
    {
        if self.has_service::<T>().await {
            self.inner.services.write().await.remove(&TypeId::of::<T>());
        }
    }

    /// Get the service from cache or initialize it
    ///
    /// If service doesn't have factory method, it will return Error. If you need to
    /// get service that doesn't have factory function, you can add it manually using
    /// `add_service` method.
    ///
    /// If you sure that service is already initialized, you can use `get_service_unchecked`
    /// method which will panic if service is not initialized.
    pub async fn get_service<T>(&self) -> Result<Option<Arc<T>>, Error>
    where
        T: ServiceFactory + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        // If the service is not initialized, initialize it and store in cache
        if !self.has_service::<T>().await {
            debug!(name = type_name::<T>(), "Initializing service");
            let service = T::factory(self.clone()).await?;

            return Ok(Some(self.add_service::<T>(service).await));
        }

        let services = self.inner.services.read().await;
        let Some(service) = services.get(&type_id) else {
            return Ok(None);
        };

        Ok(Some(service.clone().downcast::<T>().unwrap()))
    }

    /// Get the service from cache or panic if service is not initialized
    pub async fn get_service_unchecked<T>(&self) -> Arc<T>
    where
        T: ServiceFactory + Send + Sync + 'static,
    {
        self.get_service::<T>()
            .await
            .expect(format!("Failed to initialize service: {}", type_name::<T>()).as_str())
            .expect(format!("Failed to get service: {}", type_name::<T>()).as_str())
    }

    /// Warm up the service by initializing it
    ///
    /// This method will initialize the service and store it in cache. Internally
    /// it will call `get_service` method.
    pub async fn warm_up<T>(&self)
    where
        T: ServiceFactory + Send + Sync + 'static,
    {
        match self.get_service::<T>().await {
            Ok(result) => {
                if result.is_none() {
                    panic!("Failed to warm up service: {}", type_name::<T>());
                }
            }
            Err(e) => {
                panic!("Failed to warm up service {}: {}", type_name::<T>(), e);
            }
        }
    }
}

#[async_trait]
pub trait ServiceFactory {
    async fn factory(services: ServiceProvider) -> Result<Self, Error>
    where
        Self: Sized;
}
