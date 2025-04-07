use crate::data::app::{AppState, EndpointHandler};
use crate::errors::AppError;
use actix_web::{web, HttpResponse, Responder, Result as ActixResult};
use anyhow::Result;
use log::{info, warn};
use openapiv3::{OpenAPI, Operation, ReferenceOr, Response};
use serde_json::Value;
use std::path::Path;
use std::{collections::HashMap, sync::Arc};

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// Endpoint to show the loaded OpenAPI spec
async fn serve_openapi_yaml(app_state: web::Data<Arc<AppState>>) -> ActixResult<HttpResponse> {
    let yaml_content = serde_yaml::to_string(&app_state.openapi_spec)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(yaml_content))
}

pub async fn api_redirect(
    req: actix_web::HttpRequest,
    body: web::Bytes,
    app_state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let path = req.uri().path().trim_start_matches("/api");
    let method = req.method().as_str().to_lowercase();

    info!("API redirect: {} {}", method, path);

    // Find matching endpoint
    for endpoint in &app_state.endpoints {
        if endpoint.method.to_lowercase() == method {
            // Check if paths match (including path params)
            if paths_match(&endpoint.path, path, &endpoint.path_params) {
                // Return the stored response with status code
                let status_code = endpoint.response_code.parse::<u16>().unwrap_or(200);

                return HttpResponse::build(
                    actix_web::http::StatusCode::from_u16(status_code).unwrap(),
                )
                .content_type("application/json")
                .json(&endpoint.response_body);
            }
        }
    }

    // If no matching endpoint found
    HttpResponse::NotFound().json(serde_json::json!({
        "error": "Endpoint not found",
        "path": path,
        "method": method,
    }))
}

pub async fn swagger_ui() -> ActixResult<HttpResponse> {
    // TODO: This is bolierplate from AI chat maybe a more elegant solution can be used ...
    let html = r#"<!DOCTYPE html>
    <html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Swagger UI</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@4.5.0/swagger-ui.css" />
    <link rel="icon" type="image/png" href="https://unpkg.com/swagger-ui-dist@4.5.0/favicon-32x32.png" sizes="32x32" />
    <link rel="icon" type="image/png" href="https://unpkg.com/swagger-ui-dist@4.5.0/favicon-16x16.png" sizes="16x16" />
    <style>
        html { box-sizing: border-box; overflow: -moz-scrollbars-vertical; overflow-y: scroll; }
        *, *:before, *:after { box-sizing: inherit; }
        body { margin: 0; background: #fafafa; }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@4.5.0/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@4.5.0/swagger-ui-standalone-preset.js"></script>
    <script>
    window.onload = function() {
        const ui = SwaggerUIBundle({
            url: "/api/openapi.json",
            // Use our API server URL for requests
            // This makes "Try it out" in Swagger UI work with our mock server
            requestInterceptor: (req) => {
                // Rewrite URLs to use our API endpoints
                if (req.url.startsWith('http://') || req.url.startsWith('https://')) {
                    const url = new URL(req.url);
                    const path = url.pathname;
                    // Rewrite to use our /api prefix
                    req.url = '/api' + path;
                }
                return req;
            },
            dom_id: '#swagger-ui',
            deepLinking: true,
            presets: [
                SwaggerUIBundle.presets.apis,
                SwaggerUIStandalonePreset
            ],
            plugins: [
                SwaggerUIBundle.plugins.DownloadUrl
            ],
            layout: "StandaloneLayout"
        });
        window.ui = ui;
    };
    </script>
</body>
</html>"#;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

pub async fn show_openapi_spec(app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let spec_json = serde_json::to_value(&app_state.openapi_spec).unwrap_or(serde_json::json!({
        "error": "Failed to serialize OpenAPI spec"
    }));

    HttpResponse::Ok().json(spec_json)
}

pub async fn list_endpoints(app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let endpoints: Vec<serde_json::Value> = app_state
        .endpoints
        .iter()
        .map(|ep| {
            serde_json::json!({
                "path": ep.path,
                "method": ep.method,
                "status_code": ep.response_code,
            })
        })
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "endpoints": endpoints,
        "count": endpoints.len(),
    }))
}

pub async fn dynamic_handler(
    req_path: web::Path<(String, String)>, // Path and method
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<HashMap<String, String>>,
    path_params: web::Path<HashMap<String, String>>,
    req_body: Option<web::Json<Value>>,
) -> impl Responder {
    let (path_str, method_str) = req_path.into_inner();
    let method_str = method_str.to_lowercase();

    info!("Handling request: {} {}", method_str, path_str);

    for endpoint in &app_state.endpoints {
        if endpoint.method.to_lowercase() == method_str {
            // Check if the path matches
            if paths_match(&endpoint.path, &path_str, &endpoint.path_params) {
                let status_code = endpoint.response_code.parse::<u16>().unwrap_or(200);

                // In a more advance implementation, we could modify the response
                // based on the query parameters, path parameters, and request body

                return HttpResponse::build(
                    actix_web::http::StatusCode::from_u16(status_code).unwrap(),
                )
                .content_type("application/json")
                .json(&endpoint.response_body);
            }
        }
    }
    // If no matching endpoint is found, return a 404 Not Found response
    HttpResponse::NotFound().json(serde_json::json!({
        "error": "Endpoint not found",
        "path": path_str,
        "method": method_str,
    }))
}

fn paths_match(api_path: &str, request_path: &str, path_params: &[String]) -> bool {
    // Convert API path template to a regex pattern
    // For example: /users/{id} -> /users/[^/]+
    let mut pattern = api_path.to_string();

    for param in path_params {
        let param_pattern = format!("{{{}}}", param);
        pattern = pattern.replace(&param_pattern, "[^/]+");
    }

    // Escape regex special characters
    let pattern = pattern
        .replace(".", "\\.")
        .replace("?", "\\?")
        .replace("+", "\\+")
        .replace("*", "\\*")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("[", "\\[")
        .replace("]", "\\]");

    // Add start and end anchors
    let pattern = format!("^{}$", pattern);

    // Try to match
    match regex::Regex::new(&pattern) {
        Ok(re) => re.is_match(request_path),
        Err(_) => false, // If regex fails, consider it a mismatch
    }
}

pub fn build_endpoints_from_spec(spec_path: &Path) -> Result<(Vec<EndpointHandler>), AppError> {
    let yaml_content = std::fs::read_to_string(spec_path)?;

    // Parse the YAML into OpenAPI spec
    let openapi_spec: OpenAPI = serde_yaml::from_str(&yaml_content)?;
    let mut endpoints = Vec::new();

    info!(
        "Processiong OpenAPI spec with {} paths",
        openapi_spec.paths.paths.len()
    );

    // Process each path and its operations
    for (path, path_item) in &openapi_spec.paths.paths {
        let path_item = match path_item {
            ReferenceOr::Item(item) => item,
            ReferenceOr::Reference { .. } => {
                warn!("References not supported yet, skipping path: {}", path);
                continue;
            }
        };

        // FIXME: This is a temporary fix to handle paths with parameters,
        //        maybe there is a cleaner way to do this.

        // Process GET operations
        if let Some(op) = &path_item.get {
            process_operation(path, "get", op, &mut endpoints);
        }

        // Process POST operations
        if let Some(op) = &path_item.post {
            process_operation(path, "post", op, &mut endpoints);
        }

        // Process PUT operations
        if let Some(op) = &path_item.put {
            process_operation(path, "put", op, &mut endpoints);
        }

        if let Some(op) = &path_item.delete {
            process_operation(path, "delete", op, &mut endpoints);
        }

        // TODO: Process other HTTP methods (PATCH, OPTIONS, etc.)
    }
    Ok(endpoints)
}

fn process_operation(
    path: &str,
    method: &str,
    operation: &Operation,
    endpoints: &mut Vec<EndpointHandler>,
) {
    // Extract path parameters from the path
    let mut path_params = Vec::new();
    let re = regex::Regex::new(r"\{([^}]+)\}").unwrap();
    for cap in re.captures_iter(path) {
        path_params.push(cap[1].to_string());
    }
    for (status_code, response_or_ref) in &operation.responses.responses {
        let response = match response_or_ref {
            ReferenceOr::Item(reponse) => reponse,
            ReferenceOr::Reference { .. } => {
                warn!("References not supported yet, skipping",);
                continue;
            }
        };

        // Generate stub response based on schema or examples
        let stub_response = generate_stub_response(response);

        endpoints.push(EndpointHandler {
            path: path.to_string(),
            method: method.to_string(),
            response_code: status_code.to_string(),
            response_body: stub_response.to_string(),
            path_params: path_params.clone(),
        });

        info!(
            "Added endpoint: {} {} (status code: {})",
            method.to_uppercase(),
            path,
            status_code
        );
    }
}

fn generate_stub_response(response: &Response) -> Value {
    // TODO: In a real implementation, we'd use the response schema to generate
    //  a more realistic stub response. For now we'll just return a simple JSON object.
    //
    // Check if there's an example we can use
    for (content_type, media_type) in &response.content {
        if content_type.starts_with("application/json") {
            if let Some(example) = &media_type.example {
                return example.clone();
            }
        }
    }

    // default stub response
    serde_json::json!({
        "message": "This is a stub response",
        "status": "success",
    })
}
