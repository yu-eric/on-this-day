use chrono::Datelike;
use clap::{Parser, ValueEnum};
use rand::seq::SliceRandom;
use serde::Deserialize;

/// Represents a historical event with optional year information.
#[derive(Deserialize, Debug)]
struct Event {
    text: String,
    year: Option<i32>,
}

#[derive(Deserialize, Debug)]
struct OnThisDayResponse {
    selected: Option<Vec<Event>>,
    births: Option<Vec<Event>>,
    deaths: Option<Vec<Event>>,
    holidays: Option<Vec<Event>>,
    events: Option<Vec<Event>>,
}

/// Defines the possible event types the user can request.
#[derive(ValueEnum, Clone, Debug, Copy)]
enum EventType {
    All,
    Selected,
    Births,
    Deaths,
    Holidays,
    Events,
}

/// Required to convert the enum to a string for the URL.
impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}


/// Defines the command-line arguments for the application using clap.
#[derive(Parser, Debug)]
#[command(
    author = "yu-eric",
    version = "1.0",
    about = "Fetches a historical event from Wikipedia's 'On this day' page.",
    long_about = "A simple command-line tool that fetches a historical event for the current date from the official Wikipedia API. You can choose to get the oldest, newest, or a random event."
)]
struct Args {
    /// Show the oldest event of the day
    #[arg(short, long, conflicts_with = "newest", help = "Display the oldest event for today.")]
    oldest: bool,

    /// Show the newest event of the day
    #[arg(short, long, conflicts_with = "oldest", help = "Display the newest event for today.")]
    newest: bool,

    /// Filter events by a specific type
    #[arg(short = 't', long, value_enum, default_value_t = EventType::All, help = "Filter by event type.")]
    event_type: EventType,
}

/// The main entry point for the asynchronous application.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse command-line arguments provided by the user.
    let args = Args::parse();

    // 2. Get the current date using the chrono library.
    let now = chrono::Utc::now();
    let month = now.month();
    let day = now.day();

    // 3. Construct the API URL for the current date and event type.
    let event_type_str = format!("{}", args.event_type).to_lowercase();
    let url = format!(
        "https://api.wikimedia.org/feed/v1/wikipedia/en/onthisday/{}/{:02}/{:02}",
        event_type_str, month, day
    );

    println!(
        "Fetching event(s) of type '{}' for today ({:02}/{:02})...",
        event_type_str, month, day
    );

    // 4. Make an asynchronous GET request to the Wikipedia API.
    // We create a client to set a custom User-Agent. Many APIs, including
    // Wikipedia's, require a User-Agent header to identify the client application.
    // A 403 Forbidden error is common without one.
    // See: https://meta.wikimedia.org/wiki/User-Agent_policy
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "on-this-day-cli/0.1.0 (A Rust CLI tool to fetch daily historical events)")
        .send()
        .await?;

    // Check if the request was successful.
    if !response.status().is_success() {
        eprintln!(
            "Error: Failed to fetch data from Wikipedia API. Status: {}",
            response.status()
        );
        return Ok(());
    }

    // 5. Deserialize the JSON response into our Rust structs.
    let api_data: OnThisDayResponse = response.json().await?;

    // 6. Collect all events from the response into a single vector.
    // If a specific type was requested, only that list will be populated.
    // If 'all' was requested, this will combine events from all categories.
    let mut events_to_process: Vec<Event> = Vec::new();
    if let Some(mut e) = api_data.selected { events_to_process.append(&mut e); }
    if let Some(mut e) = api_data.births { events_to_process.append(&mut e); }
    if let Some(mut e) = api_data.deaths { events_to_process.append(&mut e); }
    if let Some(mut e) = api_data.holidays { events_to_process.append(&mut e); }
    if let Some(mut e) = api_data.events { events_to_process.append(&mut e); }


    if events_to_process.is_empty() {
        println!("No historical events found for today with the selected type.");
        return Ok(());
    }

    // 7. Select an event based on the command-line flags.
    // The `Option<&Event>` type indicates that we might not find an event.
    let selected_event: Option<&Event> = if args.oldest {
        // Find the event with the minimum year, ignoring events without a year.
        events_to_process
            .iter()
            .filter(|e| e.year.is_some())
            .min_by_key(|event| event.year)
    } else if args.newest {
        // Find the event with the maximum year, ignoring events without a year.
        events_to_process
            .iter()
            .filter(|e| e.year.is_some())
            .max_by_key(|event| event.year)
    } else {
        // Default behavior: select a random event.
        let mut rng = rand::thread_rng();
        events_to_process.choose(&mut rng)
    };

    // 8. Print the selected event to the console.
    if let Some(event) = selected_event {
        println!("\n--- On This Day: {:02}/{:02} ---", month, day);
        if let Some(year) = event.year {
            println!("\nYear {}: {}", year, event.text);
        } else {
            // For events without a year, like holidays
            println!("\n{}", event.text);
        }
    } else {
        // This is a fallback, e.g. if --oldest is used with --event-type holidays
        eprintln!("Could not select an event from the available data.");
    }

    Ok(())
}

