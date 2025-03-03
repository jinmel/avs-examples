use crate::services::oracle_service;
use reqwest::Error;
use std::str::FromStr;
use std::env;
use anyhow::Result;
use crate::handlers::openai::{OpenAIAgent, StableYieldFarmingAgent, Message, Agent};

pub async fn validate(proof_of_task: &str) -> Result<bool, String> {
    // Convert the proofOfTask string into a float
    let task_result = match f64::from_str(proof_of_task) {
        Ok(val) => val,
        Err(_) => return Err("Invalid proofOfTask value".to_string()),
    };

    // Fetch price details from the Oracle service
    match oracle_service::get_price("ETHUSDT").await {
        Ok(oracle_data) => {
            // Parse price from the oracle response
            let price_float = match f64::from_str(&oracle_data.price) {
                Ok(val) => val,
                Err(_) => return Err("Invalid price data from Oracle".to_string()),
            };

            // Define upper and lower bounds
            let upper_bound = price_float * 1.05;
            let lower_bound = price_float * 0.95;

            // Approve or reject based on price bounds
            let is_approved = task_result <= upper_bound && task_result >= lower_bound;
            Ok(is_approved)
        }
        Err(e) => Err(format!("Error fetching price data: {}", e)),
    }
}

pub async fn validate_agent(input_prompt: &str, agent_response: &str) -> Result<bool, String> {
    // Get OpenAI API key from environment variables
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => return Err("OPENAI_API_KEY is not set in environment variables".to_string()),
    };

    // Create an OpenAI agent
    let openai_agent = OpenAIAgent::new(api_key, "gpt-4".to_string(), 0.7);

    // Create a StableYieldFarmingAgent with the OpenAI agent
    let farming_agent = StableYieldFarmingAgent::new(openai_agent);
    
    // Prepare messages for validation
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: input_prompt.to_string(),
        },
        Message {
            role: "assistant".to_string(),
            content: agent_response.to_string(),
        },
        Message {
            role: "user".to_string(),
            content: "Is this response accurate, helpful, and following best practices for yield farming? Respond with only 'yes' or 'no'.".to_string(),
        },
    ];
    
    // Get validation from the agent
    match farming_agent.chat(messages).await {
        Ok(response) => {
            let validation_result = response.response.trim().to_lowercase();
            if validation_result == "yes" {
                Ok(true)
            } else if validation_result == "no" {
                Ok(false)
            } else {
                Err(format!("Unexpected validation response: {}", validation_result))
            }
        },
        Err(e) => Err(format!("Error during agent validation: {}", e)),
    }
}
