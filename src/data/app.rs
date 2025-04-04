use std::path::Path;

use openapiv3::OpenAPI;

pub struct EndpointHandler {
    pub path: String,
    pub method: String,
    pub response_code: String,
    pub response_body: String,
    pub path_params: Vec<String>,
}

pub struct AppState {
    pub endpoints: Vec<EndpointHandler>,
    pub openapi_spec: OpenAPI,
}

impl AppState {
    pub fn new(endpoints: Vec<EndpointHandler>, openapi_spec: OpenAPI) -> Self {
        AppState {
            endpoints,
            openapi_spec,
        }
    }

    pub fn new_with_spec_path(endpoints: Vec<EndpointHandler>, openapi_spec_file: &Path) -> Self {
        let openapi_spec = Self::get_openapi_spec(openapi_spec_file);
        AppState {
            endpoints,
            openapi_spec,
        }
    }

    fn get_openapi_spec(path: &Path) -> OpenAPI {
        let yaml_content = std::fs::read_to_string(path).expect("Failed to read spec file");
        let openapi_spec: OpenAPI =
            serde_yaml::from_str(&yaml_content).expect("Failed to parse spec");
        openapi_spec
    }

    pub fn get_spec(&self) -> &OpenAPI {
        &self.openapi_spec
    }
}
