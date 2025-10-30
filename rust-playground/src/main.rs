use axum::{
    Error, Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use uuid::Uuid;

// Data models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Uuid,
    name: String,
    email: String,
    age: u32,
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
    age: u32,
}

#[derive(Debug, Deserialize)]
struct UpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
    age: Option<u32>,
}

// In-memory database type
type Database = Arc<RwLock<HashMap<Uuid, User>>>;

// API Response types
#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: String,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: "Success".to_string(),
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message,
        }
    }
}

// API Handlers
async fn health_check() -> Json<ApiResponse<String>> {
    info!("Health check endpoint called");
    Json(ApiResponse::success("API is running!".to_string()))
}

async fn get_all_users(State(db): State<Database>) -> Json<ApiResponse<Vec<User>>> {
    info!("Getting all users");
    let users = db.read().unwrap();
    let user_list: Vec<User> = users.values().cloned().collect();
    Json(ApiResponse::success(user_list))
}

async fn get_user_by_id(
    Path(id): Path<Uuid>,
    State(db): State<Database>,
) -> Result<Json<ApiResponse<User>>, StatusCode> {
    info!("Getting user by ID: {}", id);
    let users = db.read().unwrap();

    match users.get(&id) {
        Some(user) => Ok(Json(ApiResponse::success(user.clone()))),
        None => {
            warn!("User not found: {}", id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

async fn create_user(
    State(db): State<Database>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<User>>, StatusCode> {
    info!("Creating new user: {}", payload.name);

    let new_user = User {
        id: Uuid::new_v4(),
        name: payload.name,
        email: payload.email,
        age: payload.age,
    };

    let mut users = db.write().unwrap();
    users.insert(new_user.id, new_user.clone());

    info!("User created with ID: {}", new_user.id);
    Ok(Json(ApiResponse::success(new_user)))
}

async fn update_user(
    Path(id): Path<Uuid>,
    State(db): State<Database>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<User>>, StatusCode> {
    info!("Updating user: {}", id);

    let mut users = db.write().unwrap();

    match users.get_mut(&id) {
        Some(user) => {
            if let Some(name) = payload.name {
                user.name = name;
            }
            if let Some(email) = payload.email {
                user.email = email;
            }
            if let Some(age) = payload.age {
                user.age = age;
            }

            info!("User updated: {}", id);
            Ok(Json(ApiResponse::success(user.clone())))
        }
        None => {
            warn!("User not found for update: {}", id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

async fn delete_user(
    Path(id): Path<Uuid>,
    State(db): State<Database>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("Deleting user: {}", id);

    let mut users = db.write().unwrap();

    match users.remove(&id) {
        Some(_) => {
            info!("User deleted: {}", id);
            Ok(Json(ApiResponse::success(format!(
                "User {} deleted successfully",
                id
            ))))
        }
        None => {
            warn!("User not found for deletion: {}", id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create in-memory database
    let db: Database = Arc::new(RwLock::new(HashMap::new()));

    // Add some sample data
    {
        let mut users = db.write().unwrap();
        let sample_user = User {
            id: Uuid::new_v4(),
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            age: 30,
        };
        users.insert(sample_user.id, sample_user);

        let sample_user2 = User {
            id: Uuid::new_v4(),
            name: "Jane Smith".to_string(),
            email: "jane.smith@example.com".to_string(),
            age: 25,
        };
        users.insert(sample_user2.id, sample_user2);
    }

    // Build our application with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/users", get(get_all_users))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user_by_id))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
        .layer(CorsLayer::permissive())
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to port 3000: {}", e);
            std::process::exit(1);
        });

    info!("🚀 Server starting on http://localhost:3000");
    info!("📋 Available endpoints:");
    info!("  GET    /health          - Health check");
    info!("  GET    /users           - Get all users");
    info!("  POST   /users           - Create a new user");
    info!("  GET    /users/:id       - Get user by ID");
    info!("  PUT    /users/:id       - Update user by ID");
    info!("  DELETE /users/:id       - Delete user by ID");

    axum::serve(listener, app).await.unwrap_or_else(|e| {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    });
}
