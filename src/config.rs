// SPDX-License-Identifier: AGPL-3.0-or-later

use clap::Parser;
use serde::Deserialize;

// -- TOML config structs ----------------------------------------------

#[derive(Debug, Deserialize, Default)]
pub struct ConfigFile {
    pub server: Option<ServerConfig>,
    pub upstream: Option<UpstreamConfig>,
    pub telemetry: Option<TelemetryConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct UpstreamConfig {
    pub provider: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelemetryConfig {
    pub capacity: Option<usize>,
    pub log_path: Option<String>,
    pub batch_size: Option<usize>,
    pub flush_interval_ms: Option<u64>,
}

// -- CLI args ---------------------------------------------------------

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(long, env = "LLMK_CONFIG")]
    pub config: Option<String>,

    #[arg(long, env = "LLMK_HOST")]
    pub host: Option<String>,

    #[arg(long, env = "LLMK_PORT")]
    pub port: Option<u16>,

    #[arg(long, env = "LLMK_UPSTREAM_PROVIDER")]
    pub upstream_provider: Option<String>,

    #[arg(long, env = "LLMK_UPSTREAM_BASE_URL")]
    pub upstream_base_url: Option<String>,

    #[arg(long, env = "LLMK_TELEMETRY_CAPACITY")]
    pub telemetry_capacity: Option<usize>,

    #[arg(long, env = "LLMK_TELEMETRY_LOG_PATH")]
    pub telemetry_log_path: Option<String>,

    #[arg(long, env = "LLMK_TELEMETRY_BATCH_SIZE")]
    pub telemetry_batch_size: Option<usize>,

    #[arg(long, env = "LLMK_TELEMETRY_FLUSH_INTERVAL_MS")]
    pub telemetry_flush_interval_ms: Option<u64>,
}

// -- Resolved config --------------------------------------------------

pub struct ResolvedConfig {
    pub host: String,
    pub port: u16,
    pub upstream_provider: String,
    pub upstream_base_url: String,
    pub telemetry_capacity: usize,
    pub telemetry_log_path: String,
    pub telemetry_batch_size: usize,
    pub telemetry_flush_interval_ms: u64,
}

/// Load configuration with precedence: CLI > TOML file > defaults.
pub fn load_config() -> anyhow::Result<ResolvedConfig> {
    let args = Args::parse();

    let config_file = if let Some(path) = &args.config {
        let content = std::fs::read_to_string(path)?;
        toml::from_str::<ConfigFile>(&content)?
    } else if std::path::Path::new("config.toml").exists() {
        let content = std::fs::read_to_string("config.toml")?;
        toml::from_str::<ConfigFile>(&content)?
    } else {
        ConfigFile::default()
    };

    Ok(resolve_config_values(&args, &config_file))
}

/// Helper to resolve configurations, separated from load_config for unit testing.
pub fn resolve_config_values(args: &Args, config_file: &ConfigFile) -> ResolvedConfig {
    let resolved_provider = args
        .upstream_provider
        .clone()
        .or_else(|| {
            config_file
                .upstream
                .as_ref()
                .and_then(|u| u.provider.clone())
        })
        .unwrap_or_else(|| "mock".to_string());

    let default_base_url = match resolved_provider.as_str() {
        "ollama" => "http://127.0.0.1:11434".to_string(),
        _ => "http://127.0.0.1:9000".to_string(),
    };

    let resolved_base_url = args
        .upstream_base_url
        .clone()
        .or_else(|| {
            config_file
                .upstream
                .as_ref()
                .and_then(|u| u.base_url.clone())
        })
        .unwrap_or(default_base_url);

    ResolvedConfig {
        host: args
            .host
            .clone()
            .or_else(|| config_file.server.as_ref().and_then(|s| s.host.clone()))
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        port: args
            .port
            .or_else(|| config_file.server.as_ref().and_then(|s| s.port))
            .unwrap_or(8080),
        upstream_provider: resolved_provider,
        upstream_base_url: resolved_base_url,
        telemetry_capacity: args
            .telemetry_capacity
            .or_else(|| config_file.telemetry.as_ref().and_then(|t| t.capacity))
            .unwrap_or(65536),
        telemetry_log_path: args
            .telemetry_log_path
            .clone()
            .or_else(|| {
                config_file
                    .telemetry
                    .as_ref()
                    .and_then(|t| t.log_path.clone())
            })
            .unwrap_or_else(|| "telemetry_events.jsonl".to_string()),
        telemetry_batch_size: args
            .telemetry_batch_size
            .or_else(|| config_file.telemetry.as_ref().and_then(|t| t.batch_size))
            .unwrap_or(1000),
        telemetry_flush_interval_ms: args
            .telemetry_flush_interval_ms
            .or_else(|| {
                config_file
                    .telemetry
                    .as_ref()
                    .and_then(|t| t.flush_interval_ms)
            })
            .unwrap_or(500),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_config_defaults_mock() {
        let args = Args {
            config: None,
            host: None,
            port: None,
            upstream_provider: None,
            upstream_base_url: None,
            telemetry_capacity: None,
            telemetry_log_path: None,
            telemetry_batch_size: None,
            telemetry_flush_interval_ms: None,
        };
        let config_file = ConfigFile::default();
        let resolved = resolve_config_values(&args, &config_file);

        assert_eq!(resolved.upstream_provider, "mock");
        assert_eq!(resolved.upstream_base_url, "http://127.0.0.1:9000");
    }

    #[test]
    fn test_resolve_config_defaults_ollama() {
        let args = Args {
            config: None,
            host: None,
            port: None,
            upstream_provider: Some("ollama".to_string()),
            upstream_base_url: None,
            telemetry_capacity: None,
            telemetry_log_path: None,
            telemetry_batch_size: None,
            telemetry_flush_interval_ms: None,
        };
        let config_file = ConfigFile::default();
        let resolved = resolve_config_values(&args, &config_file);

        assert_eq!(resolved.upstream_provider, "ollama");
        assert_eq!(resolved.upstream_base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn test_resolve_config_explicit_base_url_override() {
        let args = Args {
            config: None,
            host: None,
            port: None,
            upstream_provider: Some("ollama".to_string()),
            upstream_base_url: Some("http://custom-ollama:12345".to_string()),
            telemetry_capacity: None,
            telemetry_log_path: None,
            telemetry_batch_size: None,
            telemetry_flush_interval_ms: None,
        };
        let config_file = ConfigFile::default();
        let resolved = resolve_config_values(&args, &config_file);

        assert_eq!(resolved.upstream_provider, "ollama");
        assert_eq!(resolved.upstream_base_url, "http://custom-ollama:12345");
    }
}
