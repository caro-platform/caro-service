use serde::Serialize;

use krossbar_bus_common::errors::Error as BusError;
use krossbar_bus_lib::{signal::Signal as BusSignal, Result as BusResult};

pub struct Signal<T: Serialize> {
    internal: Option<BusSignal<T>>,
}

impl<T: Serialize + 'static> Signal<T> {
    pub fn new() -> Self {
        Self { internal: None }
    }

    pub async fn register(&mut self, service_name: &str, signal_name: &str) -> BusResult<()> {
        if self.internal.is_some() {
            return Err(Box::new(BusError::AlreadyRegistered));
        }

        match crate::service::SERVICE_HANDLES
            .lock()
            .await
            .get_mut(service_name)
        {
            Some(handle) => self.internal = Some(handle.bus.register_signal(signal_name)?),
            _ => panic!("Not registered"),
        }

        Ok(())
    }

    pub fn emit(&self, value: T) {
        match self.internal {
            None => panic!("Not registered"),
            Some(ref internal) => internal.emit(value),
        }
    }
}
