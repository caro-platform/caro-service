use std::{future::Future, sync::Arc};

use async_trait::async_trait;
use krossbar_bus_lib::{self, Result as BusResult};
use serde::de::DeserializeOwned;

pub trait PeerName {
    fn peer_name() -> &'static str;
}

#[async_trait]
pub trait Peer {
    async fn register(&mut self, service_name: &str) -> krossbar_bus_lib::Result<()>;

    async fn register_peer(
        service_name: &str,
        peer_name: &str,
    ) -> BusResult<Arc<krossbar_bus_lib::peer::Peer>> {
        match crate::service::SERVICE_HANDLES
            .lock()
            .await
            .get_mut(service_name)
        {
            Some(crate::service::ServiceHandle { bus, connections }) => {
                if connections.contains_key(peer_name) {
                    panic!("Peer already registered")
                }

                let peer_internal = Arc::new(bus.connect_await(peer_name).await?);
                connections.insert(peer_name.into(), peer_internal.clone());

                Ok(peer_internal)
            }
            _ => panic!("Not registered"),
        }
    }
}

#[async_trait]
pub trait PeerSignalsAndStates {
    async fn register_callbacks(&mut self, service_name: &str) -> BusResult<()>;

    async fn subscribe_on_signal<T, Ret>(
        service_name: &str,
        peer_name: &str,
        signal_name: &str,
        callback: impl Fn(T) -> Ret + Send + Sync + 'static,
    ) -> BusResult<()>
    where
        T: DeserializeOwned + Send + 'static,
        Ret: Future<Output = ()> + Send,
    {
        let maybe_peer = crate::service::SERVICE_HANDLES
            .lock()
            .await
            .get(service_name)
            .and_then(|handle| handle.connections.get(peer_name.into()))
            .cloned();

        match maybe_peer {
            Some(peer) => {
                peer.subscribe(signal_name, callback).await?;
            }
            _ => panic!("Not registered"),
        }

        Ok(())
    }

    async fn watch_state<T, Ret>(
        service_name: &str,
        peer_name: &str,
        state_name: &str,
        callback: impl Fn(T) -> Ret + Send + Sync + 'static,
    ) -> BusResult<T>
    where
        T: DeserializeOwned + Send + 'static,
        Ret: Future<Output = ()> + Send,
    {
        let maybe_peer = crate::service::SERVICE_HANDLES
            .lock()
            .await
            .get(service_name)
            .and_then(|handle| handle.connections.get(peer_name.into()))
            .cloned();

        match maybe_peer {
            Some(peer) => {
                return Ok(peer.watch(state_name, callback).await?);
            }
            _ => panic!("Not registered"),
        }
    }
}
