use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use log::{info, error};
use crate::services::validation_service;
use std::env;
use crate::handlers::openai::{OpenAIAgent, StableYieldFarmingAgent};
use serde_json::Value;

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub proofOfTask: String,
}

#[derive(Serialize)]
pub struct CustomResponse {
    pub data: serde_json::Value,
    pub message: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub data: serde_json::Value,
    pub error: bool,
    pub message: String,
}

impl CustomResponse {
    pub fn new(data: serde_json::Value, message: &str) -> Self {
        CustomResponse {
            data,
            message: message.to_string(),
        }
    }
}

impl ErrorResponse {
    pub fn new(data: serde_json::Value, message: &str) -> Self {
        ErrorResponse {
            data,
            error: true, // set error flag to true
            message: message.to_string(),
        }
    }
}

// Handler for the `validate` endpoint
pub async fn validate_task(request: web::Json<ValidateRequest>) -> impl Responder {
    let proof_of_task = &request.proofOfTask;

    info!("proofOfTask: {}", proof_of_task);

    match validation_service::validate(&proof_of_task).await {
        Ok(result) => {
            info!("Vote: {}", if result { "Approve" } else { "Not Approved" });

            let response = CustomResponse::new(
                json!({ "result": result }),
                "Task validated successfully",
            );

            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            error!("Validation error: {}", err);
            
            let response = ErrorResponse::new(
                json!({}),
                "Error during validation step",
            );
            
            HttpResponse::InternalServerError().json(response)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ValidateAgentRequest {
    pub prices: String,
    pub portfolio: String,
    pub model_name: String,
    pub task_definition_id: i32,
    pub agent_response: String,
}

pub async fn validate_agent_task(request: web::Json<ValidateAgentRequest>) -> impl Responder {
    info!("Validating agent response for task: {}", request.task_definition_id);
    
    // Get OpenAI API key from environment variables
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            error!("OPENAI_API_KEY is not set in environment variables");
            let response = ErrorResponse::new(
                json!({
                    "task_definition_id": request.task_definition_id,
                    "model_name": request.model_name
                }),
                "OpenAI API key not configured",
            );
            return HttpResponse::InternalServerError().json(response);
        }
    };

    // Create an OpenAI agent using the model_name from the request
    let openai_agent = OpenAIAgent::new(api_key, request.model_name.clone(), 0.7);

    // Create a StableYieldFarmingAgent with the OpenAI agent
    let farming_agent = StableYieldFarmingAgent::new(openai_agent);
    
    // Get a farming strategy using the agent
    match farming_agent.get_farming_strategy(&request.prices, &request.portfolio).await {
        Ok(strategy_response) => {
            // Clean up both responses by removing whitespace for comparison
            let agent_response_clean = request.agent_response.trim().to_string();
            let strategy_response_clean = strategy_response.response.trim().to_string();
            
            // Calculate similarity score (percentage of matching characters)
            let similarity_score = if !agent_response_clean.is_empty() && !strategy_response_clean.is_empty() {
                // Simple length comparison as a basic similarity metric
                let min_len = agent_response_clean.len().min(strategy_response_clean.len());
                let max_len = agent_response_clean.len().max(strategy_response_clean.len());
                (min_len as f64 / max_len as f64) * 100.0
            } else {
                0.0
            };
            
            // Define a threshold for similarity (50% similarity required)
            const SIMILARITY_THRESHOLD: f64 = 50.0;
            
            // Consider the response valid if it's not empty and meets the similarity threshold
            let is_valid = !agent_response_clean.is_empty() && similarity_score >= SIMILARITY_THRESHOLD;
            
            info!("Agent validation result: {}", if is_valid { "Approved" } else { "Not Approved" });
            info!("Similarity score: {:.2}%, threshold: {:.2}%", similarity_score, SIMILARITY_THRESHOLD);
            
            let response = CustomResponse::new(
                json!({ 
                    "result": is_valid,
                    "task_definition_id": request.task_definition_id,
                    "model_name": request.model_name,
                    "validation_details": {
                        "similarity_score": similarity_score,
                        "threshold": SIMILARITY_THRESHOLD,
                        "meets_threshold": similarity_score >= SIMILARITY_THRESHOLD
                    }
                }),
                "Agent response validated successfully",
            );
            
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            error!("Error generating validation strategy: {}", err);
            
            let response = ErrorResponse::new(
                json!({
                    "task_definition_id": request.task_definition_id,
                    "model_name": request.model_name
                }),
                &format!("Error during strategy generation: {}", err),
            );
            
            HttpResponse::InternalServerError().json(response)
        }
    }
}
