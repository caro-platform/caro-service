use std::{pin::Pin, time::Duration};

use karo_service::{
    peer::{PeerName, PeerSignalsAndStates},
    service::ServiceMethods,
    Peer as KaroPeer, Service as KaroService,
};
use log::LevelFilter;

use async_trait::async_trait;
use karo_derive::{method, peer_impl, service_impl, signal, state, Peer, Service};
use karo_service::Method;

#[derive(Peer)]
#[peer(name = "com.examples.service", features=["subscriptions"])]
struct PeerExample {
    #[method]
    pub hello_method: Method<i32, String>,
}

#[peer_impl]
impl PeerExample {
    pub fn new() -> Self {
        Self {
            hello_method: Method::new(),
        }
    }

    #[signal]
    async fn signal(&mut self, value: String) {
        println!("Signal emitted: {}", value);
    }

    #[state]
    async fn state(&mut self, value: i32) {
        println!("State changed: {}", value);
    }
}

#[derive(Service)]
#[service(name = "com.examples.client", features=["methods"])]
struct ServiceExample {
    #[peer]
    pub peer: Pin<Box<PeerExample>>,
}

#[service_impl]
impl ServiceExample {
    pub fn new() -> Self {
        Self {
            peer: Box::pin(PeerExample::new()),
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Warn)
        .init();

    let mut service = Box::pin(ServiceExample::new());

    service.register_service().await.unwrap();

    loop {
        println!(
            "Method response: {}",
            service.peer.hello_method.call(&42).await.unwrap()
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
