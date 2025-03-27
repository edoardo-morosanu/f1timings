use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::Path,
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
    name: String, // Changed from driver_name to name to match frontend
    team: String,
    time: String,
}

#[derive(Debug, Deserialize)]
struct LapTimeDeleteInput {
    name: String, // Changed from driver_name to name to match frontend
    time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppData {
    drivers: HashMap<String, Driver>,
    track_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TrackNameInput {
    name: String, // Changed from track_name to name to match the frontend
}

#[derive(Debug, Serialize)]
struct TrackNameResponse {
    name: String,
}

#[derive(Debug, Serialize)]
struct ExportResponse {
    success: bool,
    filename: String,
    message: String,
}

type AppState = Arc<Mutex<AppData>>;

async fn get_drivers(State(state): State<AppState>) -> impl IntoResponse {
    let app_data = state.lock().unwrap();
    (StatusCode::OK, Json(app_data.drivers.clone()))
}

async fn get_track_name(State(state): State<AppState>) -> impl IntoResponse {
    let app_data = state.lock().unwrap();
    let track_name = app_data.track_name.clone().unwrap_or_default();
    (StatusCode::OK, Json(TrackNameResponse { name: track_name }))
}

async fn set_track_name(
    State(state): State<AppState>,
    Json(input): Json<TrackNameInput>,
) -> impl IntoResponse {
    let mut app_data = state.lock().unwrap();
    app_data.track_name = Some(input.name.clone());

    (StatusCode::OK, Json(TrackNameResponse { name: input.name }))
}

async fn add_lap_time(
    State(state): State<AppState>,
    Json(input): Json<LapTimeInput>,
) -> impl IntoResponse {
    let mut app_data = state.lock().unwrap();

    let driver = app_data
        .drivers
        .entry(input.name.clone())
        .or_insert(Driver {
            name: input.name.clone(),
            lap_times: Vec::new(),
            team: input.team.clone(),
        });

    // Add the new lap time (we'll filter later to keep only the fastest)
    let new_lap = LapTime {
        time: input.time.clone(),
        is_fastest: false, // Will be calculated later
    };

    // If team got changed, update it
    if driver.team != input.team {
        driver.team = input.team.clone();
    }

    // Add the new lap time
    driver.lap_times.push(new_lap);

    // Keep only fastest lap time per driver (as requested)
    retain_fastest_lap_times(&mut app_data.drivers);

    // Update fastest laps across all drivers
    update_fastest_laps(&mut app_data.drivers);

    (StatusCode::OK, Json(app_data.drivers.clone()))
}

async fn delete_lap_time(
    State(state): State<AppState>,
    Json(input): Json<LapTimeDeleteInput>,
) -> impl IntoResponse {
    let mut app_data = state.lock().unwrap();

    // Check if driver exists
    if let Some(driver) = app_data.drivers.get_mut(&input.name) {
        // Remove the lap time
        driver.lap_times.retain(|lap| lap.time != input.time);

        // If driver has no more lap times, remove the driver
        if driver.lap_times.is_empty() {
            app_data.drivers.remove(&input.name);
        }

        // Update fastest laps
        update_fastest_laps(&mut app_data.drivers);

        (StatusCode::OK, Json("Lap time deleted"))
    } else {
        (StatusCode::NOT_FOUND, Json("Driver not found"))
    }
}

async fn export_lap_times(State(state): State<AppState>) -> impl IntoResponse {
    let app_data = state.lock().unwrap();

    if let Some(track_name) = &app_data.track_name {
        // Create both CSV and JSON exports
        match export_to_files(&app_data.drivers, track_name) {
            Ok(filename) => (
                StatusCode::OK,
                Json(ExportResponse {
                    success: true,
                    filename,
                    message: "Export successful".to_string(),
                }),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ExportResponse {
                    success: false,
                    filename: String::new(),
                    message: format!("Export failed: {}", e),
                }),
            ),
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(ExportResponse {
                success: false,
                filename: String::new(),
                message: "No track name set".to_string(),
            }),
        )
    }
}

fn retain_fastest_lap_times(drivers: &mut HashMap<String, Driver>) {
    // For each driver, keep only their fastest lap
    for (_, driver) in drivers.iter_mut() {
        if driver.lap_times.len() > 1 {
            // Find the fastest lap
            let mut fastest_time_value = f64::MAX;
            let mut fastest_index = 0;

            for (i, lap) in driver.lap_times.iter().enumerate() {
                let seconds = parse_time_to_seconds(&lap.time);
                if seconds < fastest_time_value {
                    fastest_time_value = seconds;
                    fastest_index = i;
                }
            }

            // Keep only the fastest lap
            let fastest_lap = driver.lap_times[fastest_index].clone();
            driver.lap_times.clear();
            driver.lap_times.push(fastest_lap);
        }
    }
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

fn export_to_files(
    drivers: &HashMap<String, Driver>,
    track_name: &str,
) -> Result<String, std::io::Error> {
    // Create "exports" directory if it doesn't exist
    let export_dir = Path::new("exports");
    if !export_dir.exists() {
        std::fs::create_dir_all(export_dir)?;
    }

    // Format filenames with track name
    let safe_track_name = track_name.replace(" ", "_");
    let csv_filename = format!("exports/{}_lap_times.csv", safe_track_name);
    let json_filename = format!("exports/{}_lap_times.json", safe_track_name);

    // Create a vector of driver and lap time pairs
    let mut lap_times: Vec<(&Driver, &LapTime)> = Vec::new();
    for driver in drivers.values() {
        for lap in &driver.lap_times {
            lap_times.push((driver, lap));
        }
    }

    // Sort by lap time (fastest first)
    lap_times.sort_by(|a, b| {
        let time_a = parse_time_to_seconds(&a.1.time);
        let time_b = parse_time_to_seconds(&b.1.time);
        time_a.partial_cmp(&time_b).unwrap()
    });

    // Export to CSV
    let mut csv_file = File::create(&csv_filename)?;

    // Write header
    writeln!(csv_file, "Position,Driver,Team,Time,Fastest")?;

    // Write data
    for (i, (driver, lap)) in lap_times.iter().enumerate() {
        writeln!(
            csv_file,
            "{},{},{},{},{}",
            i + 1,
            driver.name,
            driver.team,
            lap.time,
            if lap.is_fastest { "Yes" } else { "No" }
        )?;
    }

    // Export to JSON
    let mut json_file = File::create(&json_filename)?;

    // Create JSON structure
    let json_data = serde_json::json!({
        "track": track_name,
        "date": chrono::Local::now().format("%Y-%m-%d").to_string(),
        "fastest_laps": lap_times.iter().map(|(driver, lap)| {
            serde_json::json!({
                "position": lap_times.iter().position(|&(d, l)| d.name == driver.name && l.time == lap.time).unwrap() + 1,
                "driver": driver.name,
                "team": driver.team,
                "time": lap.time,
                "is_fastest": lap.is_fastest
            })
        }).collect::<Vec<_>>()
    });

    // Write JSON data
    write!(json_file, "{}", serde_json::to_string_pretty(&json_data)?)?;

    tracing::info!(
        "Exported lap times to {} and {}",
        csv_filename,
        json_filename
    );

    Ok(csv_filename)
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
            return parts[0].parse::<f64>().unwrap_or(0.0)
                + format!("0.{}", parts[1]).parse::<f64>().unwrap_or(0.0);
        }
    }

    // Just a number of seconds
    time.parse().unwrap_or(f64::MAX)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app_state: AppState = Arc::new(Mutex::new(AppData {
        drivers: HashMap::new(),
        track_name: None,
    }));

    let app = Router::new()
        .route("/api/drivers", get(get_drivers))
        .route("/api/laptime", post(add_lap_time)) // Changed from /api/lap to /api/laptime
        .route("/api/laptime", delete(delete_lap_time)) // Changed from /api/lap to /api/laptime
        .route("/api/track", get(get_track_name))
        .route("/api/track", post(set_track_name))
        .route("/api/export", get(export_lap_times))
        .nest_service("/admin", ServeDir::new("static/admin"))
        .nest_service("/display", ServeDir::new("static/display"))
        .fallback_service(ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
