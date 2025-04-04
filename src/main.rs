use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use clap::Parser;
use log::info;
use rusty_stub_api::data::app::AppState;
use rusty_stub_api::data::cli_args::Args;
use rusty_stub_api::transactions::build_endpoints_from_spec;
use rusty_stub_api::transactions::dynamic_handler;
use std::path::Path;
use std::sync::Arc;
#[actix_web::main]

async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let args = Args::parse();

    let spec_path = Path::new(&args.spec);

    if !(spec_path.exists()) {
        eprintln!("Spec file not found: {}", args.spec);
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("OpenAPI spec file not found: {}", args.spec),
        ));
    }

    let endpoints = match build_endpoints_from_spec(spec_path) {
        Ok(eps) => eps,
        Err(e) => {
            eprintln!("Error building endpoints {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ));
        }
    };

    info!("Loaded {} endpoints from OpenAPI spec", endpoints.len());

    let app_state = Arc::new(AppState::new_with_spec_path(endpoints, spec_path));

    let bind_addr = format!("{}:{}", args.host, args.port);
    info!("Starting server on {}", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(app_state.clone()))
            .route("/{method}/{path:.*}", web::to(dynamic_handler))
    })
    .bind(bind_addr)?
    .run()
    .await
}
