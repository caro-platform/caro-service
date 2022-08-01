use serde::Serialize;

use karo_bus_common::errors::Error as BusError;
use karo_bus_lib::{state::State as BusState, Result as BusResult};

pub struct State<T: Serialize> {
    internal: Option<BusState<T>>,
}

impl<T: Serialize + 'static> State<T> {
    pub fn new() -> Self {
        Self { internal: None }
    }

    pub async fn register(
        &mut self,
        service_name: &str,
        state_name: &str,
        initial_value: T,
    ) -> BusResult<()> {
        if self.internal.is_some() {
            return Err(Box::new(BusError::AlreadyRegistered));
        }

        match crate::service::SERVICE_HANDLES
            .lock()
            .await
            .get_mut(service_name)
        {
            Some(handle) => {
                self.internal = Some(handle.bus.register_state(state_name, initial_value)?)
            }
            _ => panic!("Not registered"),
        }

        Ok(())
    }

    pub fn set(&mut self, value: T) {
        match self.internal {
            None => panic!("Not registered"),
            Some(ref mut internal) => internal.set(value),
        }
    }

    pub fn get(&self) -> &T {
        match self.internal {
            None => panic!("Not registered"),
            Some(ref internal) => internal.get(),
        }
    }
}
