use std::{collections::HashMap, future::Future, sync::Arc};

use async_trait::async_trait;
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::Mutex;

use karo_bus_lib::{Bus, Result as BusResult};

pub(crate) struct ServiceHandle {
    pub bus: Bus,
    pub connections: HashMap<String, Arc<karo_bus_lib::peer::Peer>>,
}

lazy_static! {
    pub(crate) static ref SERVICE_HANDLES: Mutex<HashMap<String, ServiceHandle>> =
        Mutex::new(HashMap::new());
}

#[async_trait]
pub trait Service {
    async fn register_bus() -> BusResult<()> {
        let bus = Bus::register(Self::service_name()).await?;

        SERVICE_HANDLES.lock().await.insert(
            Self::service_name().into(),
            ServiceHandle {
                bus,
                connections: HashMap::new(),
            },
        );

        Ok(())
    }

    fn service_name() -> &'static str;
    async fn register_service(&mut self) -> BusResult<()>;
}

#[async_trait]
pub trait ServiceMethods: Send + Sync + Sized {
    async fn register_methods(&mut self, service_name: &str) -> BusResult<()>;

    async fn register_method<P, R, Ret>(
        service_name: &str,
        method_name: &str,
        callback: impl Fn(P) -> Ret + Send + Sync + 'static,
    ) -> BusResult<()>
    where
        P: DeserializeOwned + Send + 'static,
        R: Serialize + Send + 'static,
        Ret: Future<Output = R> + Send,
    {
        match SERVICE_HANDLES.lock().await.get_mut(service_name) {
            Some(ServiceHandle {
                bus,
                connections: _,
            }) => {
                bus.register_method(method_name, callback)?;
            }
            _ => panic!("Not registered"),
        }

        Ok(())
    }
}
