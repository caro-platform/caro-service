use std::{pin::Pin, time::Duration};

use async_trait::async_trait;
use karo_bus_lib::{self, Result as BusResult};
use karo_service::{
    method::Method,
    peer::{Peer, PeerSignalsAndStates},
    service::{Service, ServiceMethods},
    signal::Signal,
    state::State,
};
use log::LevelFilter;

struct PeerExample {
    method: Method<i32, String>,
}

impl PeerExample {
    pub fn new() -> Self {
        Self {
            method: Method::new(),
        }
    }
    async fn signal_callback(&mut self, value: String) {
        println!("Signal emitted: {}", value);
    }

    async fn state_callback(&mut self, value: i32) {
        println!("State changed: {}", value);
    }
}

#[async_trait]
impl Peer for Pin<Box<PeerExample>> {
    async fn register(&mut self, service_name: &str) -> karo_bus_lib::Result<()> {
        let peer = Self::register_peer(service_name, "karo.service.peer").await?;

        self.method.init("method", peer)?;
        Ok(())
    }
}

#[async_trait]
impl PeerSignalsAndStates for Pin<Box<PeerExample>> {
    async fn register_callbacks(&mut self, service_name: &str) -> BusResult<()> {
        let context = karo_service::this::This { pointer: self };

        Self::subscribe_on_signal(
            service_name,
            "com.examples.service",
            "signal",
            move |p| async move { context.get().signal_callback(p).await },
        )
        .await?;

        let current_state = Self::watch_state(
            service_name,
            "com.examples.service",
            "state",
            move |p| async move { context.get().state_callback(p).await },
        )
        .await?;
        self.state_callback(current_state).await;

        Ok(())
    }
}

struct ServiceExample {
    peer: Pin<Box<PeerExample>>,
    signal: Signal<String>,
    state: State<i32>,
    counter: i32,
}

impl ServiceExample {
    pub fn new() -> Self {
        Self {
            peer: Box::pin(PeerExample::new()),
            signal: Signal::new(),
            state: State::new(),
            counter: 0,
        }
    }

    async fn hello_method(&mut self, value: i32) -> String {
        self.counter += 1;
        format!("Hello, {}", value + self.counter)
    }
}

#[async_trait]
impl Service for Pin<Box<ServiceExample>> {
    fn service_name() -> &'static str {
        "com.examples.service"
    }

    async fn register_service(&mut self) -> karo_bus_lib::Result<()> {
        Self::register_bus().await?;

        self.peer.register(Self::service_name()).await?;
        self.peer.register_callbacks(Self::service_name()).await?;

        self.signal.register(Self::service_name(), "signal").await?;
        self.state
            .register(Self::service_name(), "state", 0)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl ServiceMethods for Pin<Box<ServiceExample>> {
    async fn register_methods(&mut self, service_name: &str) -> karo_bus_lib::Result<()> {
        let context = karo_service::this::This { pointer: self };

        Self::register_method(service_name, "method", move |p| async move {
            context.get().hello_method(p).await
        })
        .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Trace)
        .init();

    let mut service = Box::pin(ServiceExample::new());

    service.register_service().await.unwrap();
    service
        .register_methods("com.examples.service")
        .await
        .unwrap();

    loop {
        service.signal.emit("Hello".into());
        service.state.set(42);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
