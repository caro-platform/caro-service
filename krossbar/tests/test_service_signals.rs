use std::{env, path::Path, pin::Pin, time::Duration};

use async_trait::async_trait;
use json::JsonValue;
use log::LevelFilter;
use tempdir::TempDir;
use tokio::{
    fs::OpenOptions,
    io::AsyncWriteExt,
    sync::mpsc::{self, Sender},
    time,
};

use krossbar_bus_common::HUB_SOCKET_PATH_ENV;
use krossbar_bus_hub::{args::Args, hub::Hub};
use krossbar_derive::{peer_impl, service_impl, signal, Peer, Service};
use krossbar_service::{
    peer::{PeerName, PeerSignalsAndStates},
    Peer as KrossbarPeer, Service as KrossbarService, Signal,
};

#[derive(Service)]
#[service("com.register_signal")]
struct TestServer {
    #[signal]
    signal: Signal<String>,
}

#[service_impl]
impl TestServer {
    pub fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }
}

#[derive(Peer)]
#[peer(name = "com.register_signal", features = ["subscriptions"])]
struct TestPeer {
    pub received_value: String,
}

#[peer_impl]
impl TestPeer {
    pub fn new() -> Self {
        Self {
            received_value: "".into(),
        }
    }

    #[signal]
    async fn signal(&mut self, value: String) {
        println!("Signal emitted: {}", value);
        self.received_value = value;
    }
}

#[derive(Service)]
#[service("com.subscribe_on_signal")]
struct TestClient {
    #[peer]
    pub peer: Pin<Box<TestPeer>>,
}

#[service_impl]
impl TestClient {
    pub fn new() -> Self {
        Self {
            peer: Box::pin(TestPeer::new()),
        }
    }
}

async fn start_hub(socket_path: &str, service_files_dir: &str) -> Sender<()> {
    env::set_var(HUB_SOCKET_PATH_ENV, socket_path);

    let args = Args {
        log_level: LevelFilter::Debug,
        service_files_dir: service_files_dir.into(),
    };

    // let _ = pretty_env_logger::formatted_builder()
    //     .filter_level(args.log_level)
    //     .try_init();

    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

    tokio::spawn(async move {
        let mut hub = Hub::new(args, shutdown_rx);
        hub.run().await.expect("Failed to run hub");

        println!("Shutting hub down");
    });

    println!("Succesfully started hub socket");
    shutdown_tx
}

async fn write_service_file(service_dir: &Path, service_name: &str, content: JsonValue) {
    let service_file_path = service_dir.join(format!("{}.service", service_name));

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(service_file_path.as_path())
        .await
        .expect("Failed to create service file");

    file.write_all(json::stringify(content).as_bytes())
        .await
        .expect("Failed to write service file content");
    file.flush().await.expect("Failed to flush service file");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_service_signals() {
    let socket_dir = TempDir::new("krossbar_hub_socket_dir").expect("Failed to create socket tempdir");
    let socket_path: String = socket_dir
        .path()
        .join("krossbar_hub.socket")
        .as_os_str()
        .to_str()
        .unwrap()
        .into();

    let service_dir = TempDir::new("test_signals").expect("Failed to create tempdir");

    let shutdown_tx = start_hub(
        &socket_path,
        service_dir.path().as_os_str().to_str().unwrap(),
    )
    .await;
    // Lets wait until hub starts
    time::sleep(Duration::from_millis(10)).await;

    // Create service file second
    let service_file_json = json::parse(
        r#"
            {
                "exec": "/**/*",
                "incoming_connections": ["com.subscribe_on_signal"]
            }
            "#,
    )
    .unwrap();

    let register_service_name = "com.register_signal";
    write_service_file(service_dir.path(), register_service_name, service_file_json).await;

    let mut server = Box::pin(TestServer::new());
    server.register_service().await.unwrap();

    // Create service file first
    let service_file_json = json::parse(
        r#"
        {
            "exec": "/**/*",
            "incoming_connections": []
        }
        "#,
    )
    .unwrap();

    let service_name = "com.subscribe_on_signal";
    write_service_file(service_dir.path(), service_name, service_file_json).await;

    let mut client = Box::pin(TestClient::new());
    client.register_service().await.unwrap();

    server.signal.emit("Hello".into());
    time::sleep(Duration::from_millis(10)).await;

    assert_eq!(&client.peer.received_value, "Hello");

    shutdown_tx
        .send(())
        .await
        .expect("Failed to send shutdown request to the hub");
}
