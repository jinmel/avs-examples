use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::services::dal_service; // Import from services/price.rs
use crate::services::oracle_service;  // Import from services/task.rs
use crate::handlers::openai::{OpenAIAgent, Agent, Message, StableYieldFarmingAgent};
use std::env;
use anyhow::Result;

#[derive(Deserialize)]
pub struct ExecuteTaskPayload {
    pub taskDefinitionId: Option<i32>, // optional in case it's not included in the request body
}

#[derive(Serialize)]
struct CustomResponse {
    status: String,
    data: HashMap<String, serde_json::Value>,
}

pub async fn execute_task(payload: web::Json<ExecuteTaskPayload>) -> impl Responder {
    println!("Executing Task");

    // Default taskDefinitionId to 0 if not provided
    let task_definition_id = payload.taskDefinitionId.unwrap_or(0);
    println!("task_definition_id: {}", task_definition_id);

    match oracle_service::get_price("ETHUSDT").await {
        Ok(price_data) => {
            let proof_of_task = price_data.price;
            // Send the task
            dal_service::send_task(proof_of_task.clone(), task_definition_id).await;
            HttpResponse::Ok().json("Task executed successfully".to_string()) // Return the response as JSON
        }
        Err(err) => {
            // Error fetching price
            eprintln!("Error fetching price: {}", err);
            HttpResponse::ServiceUnavailable().json("Network error occurred")
            
        }
    }
}

#[derive(Deserialize)]
pub struct ExecuteAgentPayload {
    pub taskDefinitionId: Option<i32>,
    pub prices: String,
    pub portfolio: String,
    pub model_name: String,
}

pub async fn execute_agent(payload: web::Json<ExecuteAgentPayload>) -> impl Responder {
    println!("Executing Agent");

    let task_definition_id = payload.taskDefinitionId.unwrap_or(0);
    println!("task_definition_id: {}", task_definition_id);

    // Get OpenAI API key from environment variables
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("OPENAI_API_KEY is not set in environment variables");
            return HttpResponse::InternalServerError().json("OpenAI API key not configured");
        }
    };

    // Create an OpenAI agent
    let openai_agent = OpenAIAgent::new(
        api_key,
        payload.model_name.clone(),
        0.7,
    );

    // Create a StableYieldFarmingAgent with the OpenAI agent
    let farming_agent = StableYieldFarmingAgent::new(openai_agent);

    // Call get_farming_strategy with the provided parameters
    match farming_agent.get_farming_strategy(&payload.prices, &payload.portfolio).await {
        Ok(chat_response) => {
            println!("Input prompt: {}", chat_response.input_prompt);
            println!("Agent response: {}", chat_response.response);
            
            // Send the agent task with both input prompt and response
            match dal_service::send_agent_task(
                payload.prices.clone(),
                payload.portfolio.clone(),
                payload.model_name.clone(),
                chat_response.response.clone(),
                task_definition_id
            ).await {
                Ok(_) => {
                    println!("Successfully sent agent task to DAL service");
                },
                Err(e) => {
                    eprintln!("Error sending agent task to DAL service: {}", e);
                    // Continue execution even if sending task fails
                }
            }
            
            // Return both input and output
            let mut response_data = HashMap::new();
            response_data.insert("response".to_string(), serde_json::Value::String(chat_response.response));
            
            HttpResponse::Ok().json(CustomResponse {
                status: "success".to_string(),
                data: response_data,
            })
        },
        Err(err) => {
            eprintln!("Error calling farming agent: {}", err);
            HttpResponse::ServiceUnavailable().json("Error calling farming agent")
        }
    }
}




