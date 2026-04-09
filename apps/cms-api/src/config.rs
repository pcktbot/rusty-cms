use std::{env, net::IpAddr, str::FromStr};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_host: IpAddr,
    pub bind_port: u16,
    pub database_url: Option<String>,
    pub temporal_ui_url: String,
    pub temporal_grpc_endpoint: String,
    pub temporal_namespace: String,
    pub temporal_runner_python: String,
    pub temporal_runner_start_script: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let workspace_dir = env!("CARGO_MANIFEST_DIR");
        let default_runner_script =
            format!("{workspace_dir}/../../workers/temporal_runner/start_workflow.py");

        Self {
            bind_host: env::var("CMS_API_HOST")
                .ok()
                .and_then(|value| IpAddr::from_str(&value).ok())
                .unwrap_or_else(|| "127.0.0.1".parse().expect("valid default host")),
            bind_port: env::var("CMS_API_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(4000),
            database_url: env::var("DATABASE_URL").ok(),
            temporal_ui_url: env::var("TEMPORAL_UI_URL")
                .unwrap_or_else(|_| "http://localhost:8233".to_owned()),
            temporal_grpc_endpoint: env::var("TEMPORAL_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "localhost:7233".to_owned()),
            temporal_namespace: env::var("TEMPORAL_NAMESPACE")
                .unwrap_or_else(|_| "default".to_owned()),
            temporal_runner_python: env::var("TEMPORAL_RUNNER_PYTHON")
                .unwrap_or_else(|_| "python3".to_owned()),
            temporal_runner_start_script: env::var("TEMPORAL_RUNNER_START_SCRIPT")
                .unwrap_or(default_runner_script),
        }
    }
}
