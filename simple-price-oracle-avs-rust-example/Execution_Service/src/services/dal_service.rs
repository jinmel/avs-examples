use serde::{Deserialize, Serialize};
use std::error::Error;
use reqwest::Client;
use serde_json::json;
use alloy::{
    hex::{self, encode}, primitives::{keccak256, Bytes}, signers::{k256::{ecdsa::SigningKey, elliptic_curve::generic_array::GenericArray}, local::PrivateKeySigner, Signer}
};
use alloy_sol_types::{SolValue, sol};

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Debug)]
struct Config {
    private_key: String,
    eth_rpc_url: String,
}

impl Config {
    fn new(private_key: String, eth_rpc_url: String) -> Self {
        Config {
            private_key,
            eth_rpc_url,
        }
    }
}

// Global Config instance
static mut CONFIG: Option<Config> = None;

// Set up global Config (can be called once at initialization)
pub fn init_config(private_key: String, eth_rpc_url: String) {
    unsafe {
        CONFIG = Some(Config::new(private_key, eth_rpc_url));
    }
}

pub async fn send_task(proof_of_task: String, task_definition_id: i32) -> Result<(), Box<dyn Error>> {
    // Access global Config
    let config = unsafe {
        CONFIG.as_ref().expect("Config is not initialized")
    };
    let data = "hello";
    let result = Bytes::from(data.as_bytes().to_vec());

    // let task_definition_id = 0;

    let decoded_key = hex::decode(&config.private_key).unwrap();
    println!("decoded_key {:?}", decoded_key);
    let signing_key = SigningKey::from_bytes(GenericArray::from_slice(&decoded_key)).unwrap();
    let signer = PrivateKeySigner::from_signing_key(signing_key);

    let performer_address = signer.address();
    println!("performer_address {:?}", performer_address);

    println!("Address {:?}, {:?}, {:?}, {}", proof_of_task, result, performer_address, task_definition_id );
    let my_values = (proof_of_task.to_string(), &result, performer_address, task_definition_id);

    let encoded_data = my_values.abi_encode_params();

    // println!("encoded_data {:?} ", encoded_data);
    let message_hash = keccak256(&encoded_data);
    println!("message_hash {} ", message_hash);

    let signature = signer.sign_hash(&message_hash).await?;
    let signature_bytes = signature.as_bytes();
    // let serialized_signature = encode(signature_bytes);
    let serialized_signature = format!("0x{}", encode(signature_bytes));

    let params = vec![
        json!(proof_of_task),
        json!(result),
        json!(task_definition_id),
        json!(performer_address),
        json!(serialized_signature),
    ];

    // Call the RPC method (sendTask)
    make_rpc_request(&config.eth_rpc_url, params).await?;
    
    Ok(()) 
}

/// Sends a task with proof of AI agent inference
/// 
/// This function is specifically designed for sending tasks that involve AI agent inference.
/// It includes both the input prompt and the agent's response as proof of task execution.
/// 
/// # Arguments
/// 
/// * `input_prompt` - The prompt sent to the AI agent
/// * `agent_response` - The response received from the AI agent
/// * `task_definition_id` - The ID of the task definition
/// 
pub async fn send_agent_task(
    prices: String,
    portfolio: String,
    model_name: String,
    agent_response: String,
    task_definition_id: i32
) -> Result<(), Box<dyn Error>> {
    // Access global Config
    let config = unsafe {
        CONFIG.as_ref().expect("Config is not initialized")
    };
    
    // Create a JSON object containing both input and output
    let agent_proof = json!({
        "prices": prices,
        "portfolio": portfolio,
        "model_name": model_name,
        "agent_response": agent_response
    });
    
    // Convert to string for the proof
    let proof_of_task = agent_proof.to_string();
    
    // Create result data - this could be customized based on the agent's response
    // For now, we're using the agent's response as the result data
    let result = Bytes::from(agent_response.as_bytes().to_vec());

    // Get signer and address
    let decoded_key = hex::decode(&config.private_key).unwrap();
    let signing_key = SigningKey::from_bytes(GenericArray::from_slice(&decoded_key)).unwrap();
    let signer = PrivateKeySigner::from_signing_key(signing_key);
    let performer_address = signer.address();

    println!("Agent task - prices: {}, portfolio: {}, model_name: {}, Output: {}, Address: {:?}, Task ID: {}", 
             prices, portfolio, model_name, agent_response, performer_address, task_definition_id);
    
    // Create the values tuple for encoding
    let my_values = (proof_of_task.to_string(), &result, performer_address, task_definition_id);
    let encoded_data = my_values.abi_encode_params();
    
    // Hash and sign the data
    let message_hash = keccak256(&encoded_data);
    println!("Agent task message hash: {}", message_hash);
    
    let signature = signer.sign_hash(&message_hash).await?;
    let signature_bytes = signature.as_bytes();
    let serialized_signature = format!("0x{}", encode(signature_bytes));

    // Prepare RPC parameters
    let params = vec![
        json!(proof_of_task),
        json!(result),
        json!(task_definition_id),
        json!(performer_address),
        json!(serialized_signature),
    ];

    // Call the RPC method
    make_rpc_request(&config.eth_rpc_url, params).await?;
    
    Ok(())
}

// Function for sending the RPC request
async fn make_rpc_request(rpc_url: &String, params: Vec<serde_json::Value>) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    
    println!("Sending task with params: {:?}", params);

    let body = json!({
        "jsonrpc": "2.0",
        "method": "sendTask",
        "params": params,
        "id": 1
    });

    let response = client.post(rpc_url)
        .json(&body)
        .send()
        .await?;

    // Deserialize the response
    let rpc_response: JsonRpcResponse = response.json().await?;

    // Handle the response
    if let Some(result) = rpc_response.result {
        Ok(format!("Task executed successfully with result {:?}", result)) 
    } else if let Some(error) = rpc_response.error {
        Err(format!("RPC Error {}: {}", error.code, error.message).into())
    } else {
        Err("Unknown RPC response".into())
    }
}
