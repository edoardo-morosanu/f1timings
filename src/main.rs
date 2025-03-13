use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LapTime {
    time: String,
    is_fastest: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Driver {
    name: String,
    lap_times: Vec<LapTime>,
    team: String, // "RedBull" or "McLaren"
}

#[derive(Debug, Deserialize)]
struct LapTimeInput {
    driver_name: String,
    team: String,
    time: String,
}

type AppState = Arc<Mutex<HashMap<String, Driver>>>;

async fn get_drivers(State(state): State<AppState>) -> impl IntoResponse {
    let drivers = state.lock().unwrap();
    (StatusCode::OK, Json(drivers.clone()))
}

async fn add_lap_time(
    State(state): State<AppState>,
    Json(input): Json<LapTimeInput>,
) -> impl IntoResponse {
    let mut drivers = state.lock().unwrap();

    let driver = drivers.entry(input.driver_name.clone()).or_insert(Driver {
        name: input.driver_name.clone(),
        lap_times: Vec::new(),
        team: input.team.clone(),
    });

    let new_lap = LapTime {
        time: input.time.clone(),
        is_fastest: false, // Will be calculated later
    };

    driver.lap_times.push(new_lap);

    // Update fastest laps
    update_fastest_laps(&mut drivers);

    (StatusCode::OK, Json(drivers.clone()))
}

fn update_fastest_laps(drivers: &mut HashMap<String, Driver>) {
    // Find fastest lap
    let mut fastest_time_value = f64::MAX;
    let mut fastest_time_string = String::new();

    // Reset all fastest flags and find fastest lap
    for (_, driver) in drivers.iter_mut() {
        for lap in &mut driver.lap_times {
            lap.is_fastest = false;

            // Parse time string to seconds for proper comparison
            let seconds = parse_time_to_seconds(&lap.time);
            if seconds < fastest_time_value {
                fastest_time_value = seconds;
                fastest_time_string = lap.time.clone();
            }
        }
    }

    // Set fastest lap flag
    if !fastest_time_string.is_empty() {
        for (_, driver) in drivers.iter_mut() {
            for lap in &mut driver.lap_times {
                if lap.time == fastest_time_string {
                    lap.is_fastest = true;
                }
            }
        }
    }
}

// Helper function to parse time string to seconds
fn parse_time_to_seconds(time: &str) -> f64 {
    if time.contains(':') {
        // Format: mm:ss.sss
        let parts: Vec<&str> = time.split(':').collect();
        let minutes: f64 = parts[0].parse().unwrap_or(0.0);
        let seconds: f64 = parts[1].parse().unwrap_or(0.0);
        return minutes * 60.0 + seconds;
    } else if time.contains('.') {
        // Format: mm.ss.sss or ss.sss
        let parts: Vec<&str> = time.split('.').collect();

        if parts.len() == 3 {
            // Format: mm.ss.sss
            let minutes: f64 = parts[0].parse().unwrap_or(0.0);
            let seconds: f64 = parts[1].parse().unwrap_or(0.0);
            let millis: f64 = format!("0.{}", parts[2]).parse().unwrap_or(0.0);
            return minutes * 60.0 + seconds + millis;
        } else if parts.len() == 2 {
            // Format: ss.sss
            return parts[0].parse::<f64>().unwrap_or(0.0) +
                format!("0.{}", parts[1]).parse::<f64>().unwrap_or(0.0);
        }
    }

    // Just a number of seconds
    time.parse().unwrap_or(f64::MAX)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app_state: AppState = Arc::new(Mutex::new(HashMap::new()));

    // Create a simple router without SwaggerUI
    let app = Router::new()
        .route("/api/drivers", get(get_drivers))
        .route("/api/lap", post(add_lap_time))
        .nest_service("/admin", ServeDir::new("static/admin"))
        .nest_service("/display", ServeDir::new("static/display"))
        .fallback_service(ServeDir::new("static"))  // Use fallback_service instead
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
