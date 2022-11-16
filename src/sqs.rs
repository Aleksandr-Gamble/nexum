use std::vec::Vec;
pub use aws_config;
pub use aws_sdk_sqs::{model::Message, Client, Region};
use serde::{Serialize, de::DeserializeOwned};
use serde_json;
pub use crate::core::{SimpleError, GenericError};



pub struct Messenger {
    client: Client,
    queue_url: String,

}

impl Messenger {

    /// Instantiate a new messenger
    pub async fn new(region: &'static str, queue_url: &str) -> Self {
        let config = aws_config::from_env().region(Region::new(region)).load().await;
        let client = Client::new(&config);
        let queue_url = queue_url.to_string();
        Messenger{client, queue_url}
    }

    pub async fn poll_messages(&self, delete_on_receipt: bool) -> Result<Vec<Message>, GenericError> {
        let message_batch = self.client
            .receive_message()
            .queue_url(&self.queue_url)
            .send().await?;

        let messages = message_batch.messages.unwrap_or_default();
        
        if delete_on_receipt {
            for message in &messages {
                let receipt_handle = match &message.receipt_handle {
                    Some(val) => val,
                    None => continue,
                };
                let _ = &self.client.delete_message()
                    .queue_url(&self.queue_url)
                    .receipt_handle(receipt_handle)
                    .send().await?;

            }
        }
        Ok(messages)
        
    }

    
    /// Return the body of messages as strings
    pub async fn poll_strings(&self, delete_on_receipt: bool) -> Result<Vec<String>, GenericError> {
        let messages = self.poll_messages(delete_on_receipt).await?;
        let mut resp = Vec::new();
        for message in messages {
            let body = &message.body.unwrap_or_default();
            resp.push(body.clone());
        }
        Ok(resp)
    }


    /// Return the body of messages as deserializable structs
    pub async fn poll<T: DeserializeOwned>(&self, delete_on_receipt: bool) -> Result<Vec<T>, GenericError> {
        let messages = self.poll_messages(delete_on_receipt).await?;
        let mut resp = Vec::new();
        for message in messages {
            let body = &message.body.unwrap_or_default();
            let jz: T = match serde_json::from_str(body) {
                Ok(val) => val,
                Err(_) => {
                    println!("ERROR! Unable to deserialize the desired struct from '{}'", body);
                    return Err(SimpleError{message:"JSON dserialization error".to_string()}.into())
                }
            };
            resp.push(jz)
        }
        Ok(resp)
    }



    /// publish a message (could be a string or serializable struct) to the queue with a given group_id
    pub async fn push<T: Serialize>(&self, msg: &T, group_id: &str) -> Result<String, GenericError> {
        let body = serde_json::to_string(msg)?;
        let smo = self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(body)
            .message_group_id(group_id)
            .send().await?;
        let message_id = smo
            .message_id
            .ok_or(SimpleError{message: "push request did not return a message_id!".to_string()})?;  
        Ok(message_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tokio::runtime::Runtime;

    #[test]
    fn get_a_message() {
        // In order for this test to work, you need the appropriate credentials in ~/.aws,
        // The associated IAM role needs to have permission to access the SQS Queue,
        // and you need to specify the PLUMBER_SQS_URL
        let sqs_url = env::var("PLUMBER_SQS_URL").unwrap();
        println!("SQS URL={}", &sqs_url);
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let messenger = Messenger::new("us-east-1", &sqs_url).await;
            let messages = messenger.poll_strings(false).await.unwrap();
            for message in messages {
                println!("GOT MESSAGE {}", &message);
            }

        })
    }
}