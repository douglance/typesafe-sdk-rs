//! The `typesafe` executable.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    typesafe_sdk_cli::build_cli().serve().await
}
