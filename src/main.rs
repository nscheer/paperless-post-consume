use std::env;

use regex::Regex;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use lazy_static;

const PAPERLESS_API_URL_DEFAULT: &str = "http://localhost:8000/api/";

#[derive(Debug, Serialize, Deserialize)]
struct DocumentProperties {
    title: String,
    created_date: String,
}

lazy_static::lazy_static! {
    static ref DATE_PATTERNS: [Regex; 2] = [
        // match title for ISO date
        Regex::new(r"^(?<year>[0-9]{4})-(?<month>[0-9]{2})-(?<day>[0-9]{2})\s*-?\s*").unwrap(),
        // match title for German
        Regex::new(r"^(?<day>[0-9]{2})\.(?<month>[0-9]{2})\.(?<year>[0-9]{4})\s*-?\s*").unwrap(),
    ];
}

#[tokio::main]
async fn main() {
    println!("{} - {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let document_id: i32 = env::var("DOCUMENT_ID")
        .expect("DOCUMENT_ID environment variable is not set")
        .parse()
        .expect("unable to parse DOCUMENT_ID to integer");

    let api_token = env::var("PAPERLESS_API_TOKEN")
        .expect("PAPERLESS_API_TOKEN environment variable is not set");

    let api_url = match env::var("PAPERLESS_API_URL") {
        Ok(url) => {
            println!("using provided api url: {url}");
            url
        }
        Err(_) => {
            println!("environment variable PAPERLESS_API_URL is not set, using default ({PAPERLESS_API_URL_DEFAULT})");
            PAPERLESS_API_URL_DEFAULT.to_string()
        }
    };

    println!("working on document id {document_id}");

    let request_url = format!("{api_url}documents/{document_id}/");

    let client = reqwest::Client::new();
    let response = client
        .get(&request_url)
        .header(reqwest::header::AUTHORIZATION, format!("Token {api_token}"))
        .send()
        .await
        .expect("unable to fetch document data");

    // check http return code
    match response.status() {
        StatusCode::OK => (),
        StatusCode::UNAUTHORIZED => panic!(
            "got a 401 response - it seems the api token does not work: {:#}",
            response.text().await.unwrap()
        ),
        _ => panic!(
            "something unexpected happened: {:#}",
            response.text().await.unwrap()
        ),
    }

    let document_data = response
        .json::<DocumentProperties>()
        .await
        .expect("unable to parse document data");

    println!(
        "document properties for document {document_id}: {:#?}",
        document_data
    );

    let matches = DATE_PATTERNS
        .iter()
        .find_map(|pattern| pattern.captures(&document_data.title));

    let Some(date_parts) = matches else {
        println!("no date match found - nothing to do");
        return;
    };

    let new_document_title = &document_data.title[date_parts[0].len()..];

    // contruct new document properties
    let new_document_data = DocumentProperties {
        title: new_document_title.to_string(),
        created_date: format!(
            "{}-{}-{}",
            &date_parts["year"], &date_parts["month"], &date_parts["day"]
        ),
    };

    println!(
        "new document properties for document {document_id}: {:#?}",
        new_document_data
    );

    let response = client
        .patch(&request_url)
        .header(reqwest::header::AUTHORIZATION, format!("Token {api_token}"))
        .json(&new_document_data)
        .send()
        .await
        .expect("unable to set new document properties");

    // check http return code
    match response.status() {
        StatusCode::OK => println!("successfully renamed document and updated created date"),
        StatusCode::UNAUTHORIZED => panic!(
            "got a 401 response - it seems the api token does not work: {:#}",
            response.text().await.unwrap()
        ),
        _ => panic!(
            "something unexpected happened: {:#}",
            response.text().await.unwrap()
        ),
    }
}
