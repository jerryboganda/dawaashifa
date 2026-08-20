use std::fs;
use std::path::Path;
use utoipa::OpenApi;

fn main() {
    let openapi = shifa_api::openapi::ApiDoc::openapi();
    let json = openapi
        .to_pretty_json()
        .expect("Failed to serialize OpenAPI spec");

    let contracts_dir = Path::new("contracts");
    if !contracts_dir.exists() {
        fs::create_dir_all(contracts_dir).expect("Failed to create contracts dir");
    }

    let target_path = contracts_dir.join("openapi.json");
    fs::write(&target_path, json).expect("Failed to write contracts/openapi.json");
    println!(
        "Successfully emitted OpenAPI contract to {}",
        target_path.display()
    );
}
