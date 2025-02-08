use alkanes_runtime::{
    runtime::AlkaneResponder,
    declare_alkane,
    auth::AuthenticatedResponder,
    storage::StoragePointer
};
use alkanes_support::{
    response::CallResponse
    
};
use metashrew_support::index_pointer::KeyValuePointer;
use metashrew_support::compat::to_arraybuffer_layout;
use std::println;  // For debug logging
use anyhow::Result;

#[derive(Default)]
pub struct Counter;

// Implement authenticated functions
impl AuthenticatedResponder for Counter {}

impl AlkaneResponder for Counter {
    fn execute(&self) -> Result<CallResponse> {
        // Get the context for input data
        let context = self.context()?;
        println!("Executing with context: {:?}", context);
        // First input is the operation: 
        // 0 = get count
        // 1 = increment
        // 2 = decrement
        // 3 = initialize with auth token
        let operation = context.inputs.get(0).copied().unwrap_or(0);
        println!("Operation: {}", operation);

        // Storage pointer for our counter value
        let mut counter_ptr = StoragePointer::from_keyword("/counter");
        
        let mut response = CallResponse::default();

        match operation {
            // Get current count
            0 => {
                let count = u64::from_le_bytes(
                    counter_ptr.get().as_ref()[..8].try_into()?
                );
                response.data = count.to_le_bytes().to_vec();
            },

            // Increment
            1 => {
                self.only_owner()?; // Only owner can increment
                let count = u64::from_le_bytes(
                    counter_ptr.get().as_ref()[..8].try_into()?
                );
                let new_count = count + 1;
                counter_ptr.set(std::sync::Arc::new(new_count.to_le_bytes().to_vec()));
                response.data = new_count.to_le_bytes().to_vec();
            },

            // Decrement
            2 => {
                self.only_owner()?; // Only owner can decrement
                let count = u64::from_le_bytes(
                    counter_ptr.get().as_ref()[..8].try_into()?
                );
                let new_count = count.saturating_sub(1); // Prevent underflow
                counter_ptr.set(std::sync::Arc::new(new_count.to_le_bytes().to_vec()));
                response.data = new_count.to_le_bytes().to_vec();
            },

            // Initialize with auth token
            3 => {
                // Deploy auth token if not already deployed
                if counter_ptr.get().as_ref().len() == 0 {
                    self.deploy_auth_token(1)?;
                    counter_ptr.set(std::sync::Arc::new(0u64.to_le_bytes().to_vec()));
                    response.data = vec![1]; // Success
                } else {
                    response.data = vec![0]; // Already initialized
                }
            },

            _ => {
                response.data = vec![0, 0, 0, 0, 0, 0, 0, 0]; // Invalid operation
            }
        }

        Ok(response)
    }
}

// Declare the alkane entry point
declare_alkane!(Counter);
#[cfg(test)]
mod tests {
    use super::*;
    use alkanes_support::{
        context::Context,
        id::AlkaneId,
        parcel::AlkaneTransferParcel,
    };

    #[test]
    fn test_initialization() {
        // Create a minimal context
        let context = Context {
            myself: AlkaneId { block: 2, tx: 1 },
            caller: AlkaneId { block: 0, tx: 0 },
            vout: 0,
            incoming_alkanes: AlkaneTransferParcel::default(),
            inputs: vec![3], // Operation 3 = initialize
        };

        // Initialize the contract
        let counter = Counter::default();
        counter.initialize();

        // Set context
        unsafe {
            std::ptr::write(
                &mut alkanes_runtime::imports::_CONTEXT as *mut Option<Context>,
                Some(context)
            );
        }

        // Execute
        let result = counter.execute();
        assert!(result.is_ok(), "Execution failed: {:?}", result.err());
    }
}