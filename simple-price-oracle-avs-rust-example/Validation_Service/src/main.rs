use std::env;
use actix_web::{web, App, HttpServer, Responder, middleware::Logger};
mod services;

mod handlers {
    pub mod task;
    pub mod openai;
}

// Main function
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables (if using dotenv)
    dotenv::dotenv().ok();
    
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Get the port from environment variables or default to 4003
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "4002".to_string())
        .parse()
        .expect("PORT must be a valid number");


    // Start the server
    println!("Server started on port: {}", port);
    HttpServer::new(|| {
        App::new()
        .wrap(Logger::default())
        .route("/task/validate", web::post().to(handlers::task::validate_agent_task))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
