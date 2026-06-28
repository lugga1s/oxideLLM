// SPDX-License-Identifier: AGPL-3.0-or-later

use clap::Parser;
use serde::Deserialize;

// ── TOML config structs ──────────────────────────────────────────────

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

// ── CLI args ─────────────────────────────────────────────────────────

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

// ── Resolved config ──────────────────────────────────────────────────

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

    Ok(ResolvedConfig {
        host: args
            .host
            .or_else(|| config_file.server.as_ref().and_then(|s| s.host.clone()))
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        port: args
            .port
            .or_else(|| config_file.server.as_ref().and_then(|s| s.port))
            .unwrap_or(8080),
        upstream_provider: args
            .upstream_provider
            .or_else(|| {
                config_file
                    .upstream
                    .as_ref()
                    .and_then(|u| u.provider.clone())
            })
            .unwrap_or_else(|| "mock".to_string()),
        upstream_base_url: args
            .upstream_base_url
            .or_else(|| {
                config_file
                    .upstream
                    .as_ref()
                    .and_then(|u| u.base_url.clone())
            })
            .unwrap_or_else(|| "http://127.0.0.1:9000".to_string()),
        telemetry_capacity: args
            .telemetry_capacity
            .or_else(|| config_file.telemetry.as_ref().and_then(|t| t.capacity))
            .unwrap_or(65536),
        telemetry_log_path: args
            .telemetry_log_path
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
    })
}
