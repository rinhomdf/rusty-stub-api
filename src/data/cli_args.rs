use clap::Parser;

// Command line arguments for the server
#[derive(Parser, Debug)]
#[clap(
    name = "OpenAPI Rust Server",
    about = "Generates a server from an OpenAPI spec"
)]
pub struct Args {
    /// Path to the OpenAPI YAML specification file
    #[clap(short, long, default_value = "api-spec.yaml")]
    pub spec: String,

    /// Port to listen on
    #[clap(short, long, default_value = "8080")]
    pub port: u16,

    /// Host to bind to
    #[clap(short, long, default_value = "127.0.0.1")]
    pub host: String,
}
