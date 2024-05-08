use std::{marker::PhantomData, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};

use krossbar_bus_common::errors::Error as BusError;
use krossbar_bus_lib::Result as BusResult;

pub struct Method<P: Serialize, R: DeserializeOwned> {
    method_name: String,
    peer_connection: Option<Arc<krossbar_bus_lib::peer::Peer>>,
    _pdata: PhantomData<(P, R)>,
}

impl<P: Serialize, R: DeserializeOwned> Method<P, R> {
    pub fn new() -> Self {
        Self {
            method_name: String::new(),
            peer_connection: None,
            _pdata: PhantomData,
        }
    }

    pub fn init(
        &mut self,
        method_name: &str,
        connection: Arc<krossbar_bus_lib::peer::Peer>,
    ) -> BusResult<()> {
        if self.peer_connection.is_some() {
            return Err(Box::new(BusError::AlreadyRegistered));
        }

        self.method_name = method_name.into();
        self.peer_connection = Some(connection);

        Ok(())
    }

    pub async fn call(&self, value: &P) -> BusResult<R> {
        match self.peer_connection {
            None => panic!("Not initialized"),
            Some(ref peer) => peer.call(&self.method_name, value).await,
        }
    }
}
