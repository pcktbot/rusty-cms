use anyhow::bail;
use std::{env, net::IpAddr, path::Path, str::FromStr};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_host: IpAddr,
    pub bind_port: u16,
    pub require_database: bool,
    pub database_url: Option<String>,
    pub temporal_ui_url: String,
    pub temporal_grpc_endpoint: String,
    pub temporal_namespace: String,
    pub temporal_runner_python: String,
    pub temporal_runner_start_script: String,
    pub temporal_runner_result_script: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        let workspace_dir = env!("CARGO_MANIFEST_DIR");
        let default_runner_script =
            format!("{workspace_dir}/../../workers/temporal_runner/start_workflow.py");
        let default_result_script =
            format!("{workspace_dir}/../../workers/temporal_runner/get_workflow_result.py");

        let config = Self {
            bind_host: env::var("CMS_API_HOST")
                .ok()
                .and_then(|value| IpAddr::from_str(&value).ok())
                .unwrap_or_else(|| "127.0.0.1".parse().expect("valid default host")),
            bind_port: env::var("CMS_API_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(4000),
            require_database: bool_from_env("CMS_REQUIRE_DATABASE", false),
            database_url: nonempty_env("DATABASE_URL"),
            temporal_ui_url: string_from_env("TEMPORAL_UI_URL", "http://localhost:8233"),
            temporal_grpc_endpoint: string_from_env("TEMPORAL_GRPC_ENDPOINT", "localhost:7233"),
            temporal_namespace: string_from_env("TEMPORAL_NAMESPACE", "default"),
            temporal_runner_python: string_from_env("TEMPORAL_RUNNER_PYTHON", "python3"),
            temporal_runner_start_script: env::var("TEMPORAL_RUNNER_START_SCRIPT")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(default_runner_script),
            temporal_runner_result_script: env::var("TEMPORAL_RUNNER_RESULT_SCRIPT")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(default_result_script),
        };
        config.validate()?;
        Ok(config)
    }

    #[cfg(test)]
    pub fn for_tests() -> Self {
        let workspace_dir = env!("CARGO_MANIFEST_DIR");
        let default_runner_script =
            format!("{workspace_dir}/../../workers/temporal_runner/start_workflow.py");
        let default_result_script =
            format!("{workspace_dir}/../../workers/temporal_runner/get_workflow_result.py");

        Self {
            bind_host: "127.0.0.1".parse().expect("valid test host"),
            bind_port: 4000,
            require_database: false,
            database_url: None,
            temporal_ui_url: "http://localhost:8233".to_owned(),
            temporal_grpc_endpoint: "localhost:7233".to_owned(),
            temporal_namespace: "default".to_owned(),
            temporal_runner_python: "python3".to_owned(),
            temporal_runner_start_script: default_runner_script,
            temporal_runner_result_script: default_result_script,
        }
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.require_database && self.database_url.is_none() {
            bail!("CMS_REQUIRE_DATABASE=true but DATABASE_URL is not set");
        }
        if self.temporal_ui_url.trim().is_empty() {
            bail!("TEMPORAL_UI_URL must not be empty");
        }
        if self.temporal_grpc_endpoint.trim().is_empty() {
            bail!("TEMPORAL_GRPC_ENDPOINT must not be empty");
        }
        if self.temporal_namespace.trim().is_empty() {
            bail!("TEMPORAL_NAMESPACE must not be empty");
        }
        if self.temporal_runner_python.trim().is_empty() {
            bail!("TEMPORAL_RUNNER_PYTHON must not be empty");
        }
        if self.temporal_runner_start_script.trim().is_empty() {
            bail!("TEMPORAL_RUNNER_START_SCRIPT must not be empty");
        }
        if !Path::new(&self.temporal_runner_start_script).exists() {
            bail!(
                "TEMPORAL_RUNNER_START_SCRIPT does not exist: {}",
                self.temporal_runner_start_script
            );
        }
        if self.temporal_runner_result_script.trim().is_empty() {
            bail!("TEMPORAL_RUNNER_RESULT_SCRIPT must not be empty");
        }
        if !Path::new(&self.temporal_runner_result_script).exists() {
            bail!(
                "TEMPORAL_RUNNER_RESULT_SCRIPT does not exist: {}",
                self.temporal_runner_result_script
            );
        }
        Ok(())
    }
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn string_from_env(name: &str, default: &str) -> String {
    nonempty_env(name).unwrap_or_else(|| default.to_owned())
}

fn bool_from_env(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            other => {
                let _ = other;
                default
            }
        },
        Err(_) => default,
    }
}
