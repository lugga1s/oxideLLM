// SPDX-License-Identifier: AGPL-3.0-or-later

//! Configuration parsing, validation, and resolution logic for oxideLLM.

use clap::Parser;
use serde::Deserialize;

// -- TOML config structs ----------------------------------------------

/// Configuration layout matching the TOML configuration file schema.
#[derive(Debug, Deserialize, Default)]
pub struct ConfigFile {
    /// Server hosting settings.
    pub server: Option<ServerConfig>,
    /// Legacy single upstream configuration parameter.
    pub upstream: Option<UpstreamConfig>,
    /// List of multiple configured upstreams.
    #[serde(default)]
    pub upstreams: Vec<UpstreamConfig>,
    /// Health checking intervals and timeouts.
    pub upstream_health: Option<UpstreamHealthConfig>,
    /// Telemetry queue capacity and log flush settings.
    pub telemetry: Option<TelemetryConfig>,
}

/// Server host and port configuration options.
#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// Bind address interface host.
    pub host: Option<String>,
    /// Bind address port.
    pub port: Option<u16>,
    /// Limit on maximum request body size.
    pub request_body_max_bytes: Option<usize>,
    /// TCP connection timeout to upstream servers.
    pub upstream_connect_timeout_ms: Option<u64>,
    /// Request roundtrip timeout to upstream servers.
    pub upstream_request_timeout_ms: Option<u64>,
}

/// Individual upstream target configuration options.
#[derive(Debug, Deserialize, Clone)]
pub struct UpstreamConfig {
    /// Unique identifier for the upstream.
    pub id: Option<String>,
    /// Target provider name (e.g. "ollama", "openai").
    pub provider: Option<String>,
    /// Base HTTP URL of upstream endpoint.
    pub base_url: Option<String>,
    /// Priority level (lower values take precedence).
    pub priority: Option<u16>,
    /// Custom path for health checks.
    pub health_path: Option<String>,
}

/// Upstream periodic health checker worker configuration options.
#[derive(Debug, Deserialize)]
pub struct UpstreamHealthConfig {
    /// Periodic frequency interval for ping checks.
    pub interval_ms: Option<u64>,
    /// Timeout limit for healthy response validation.
    pub timeout_ms: Option<u64>,
}

/// Telemetry channel capacity and disk sync settings.
#[derive(Debug, Deserialize)]
pub struct TelemetryConfig {
    /// In-memory queue slot count.
    pub capacity: Option<usize>,
    /// Log output destination file path.
    pub log_path: Option<String>,
    /// Maximum count of events in each batched disk flush.
    pub batch_size: Option<usize>,
    /// Periodic interval constraint for flushing remaining events to disk.
    pub flush_interval_ms: Option<u64>,
}

// -- CLI args ---------------------------------------------------------

/// Command line arguments parsed using clap.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    /// Custom path to config file.
    #[arg(long, env = "LLMK_CONFIG")]
    pub config: Option<String>,

    /// Host name to bind to.
    #[arg(long, env = "LLMK_HOST")]
    pub host: Option<String>,

    /// Port to bind to.
    #[arg(long, env = "LLMK_PORT")]
    pub port: Option<u16>,

    /// Default upstream provider name.
    #[arg(long, env = "LLMK_UPSTREAM_PROVIDER")]
    pub upstream_provider: Option<String>,

    /// Default upstream base URL.
    #[arg(long, env = "LLMK_UPSTREAM_BASE_URL")]
    pub upstream_base_url: Option<String>,

    /// Maximum size of client request payloads.
    #[arg(long, env = "LLMK_REQUEST_BODY_MAX_BYTES")]
    pub request_body_max_bytes: Option<usize>,

    /// TCP connect timeout to upstreams.
    #[arg(long, env = "LLMK_UPSTREAM_CONNECT_TIMEOUT_MS")]
    pub upstream_connect_timeout_ms: Option<u64>,

    /// Roundtrip request timeout to upstreams.
    #[arg(long, env = "LLMK_UPSTREAM_REQUEST_TIMEOUT_MS")]
    pub upstream_request_timeout_ms: Option<u64>,

    /// Upstream health check polling interval.
    #[arg(long, env = "LLMK_UPSTREAM_HEALTH_INTERVAL_MS")]
    pub upstream_health_interval_ms: Option<u64>,

    /// Upstream health check timeout.
    #[arg(long, env = "LLMK_UPSTREAM_HEALTH_TIMEOUT_MS")]
    pub upstream_health_timeout_ms: Option<u64>,

    /// In-memory buffer size for telemetry events.
    #[arg(long, env = "LLMK_TELEMETRY_CAPACITY")]
    pub telemetry_capacity: Option<usize>,

    /// File path to write analytics logs.
    #[arg(long, env = "LLMK_TELEMETRY_LOG_PATH")]
    pub telemetry_log_path: Option<String>,

    /// Max size of a single telemetry batch write.
    #[arg(long, env = "LLMK_TELEMETRY_BATCH_SIZE")]
    pub telemetry_batch_size: Option<usize>,

    /// Telemetry log sync frequency.
    #[arg(long, env = "LLMK_TELEMETRY_FLUSH_INTERVAL_MS")]
    pub telemetry_flush_interval_ms: Option<u64>,
}

// -- Resolved config --------------------------------------------------

/// Fully resolved config structure merging CLI, TOML, and default parameters.
pub struct ResolvedConfig {
    /// Bind address host interface.
    pub host: String,
    /// Bind address port.
    pub port: u16,
    /// Fallback provider label.
    pub upstream_provider: String,
    /// Fallback base URL.
    pub upstream_base_url: String,
    /// Priority ordered list of configured upstreams.
    pub upstreams: Vec<ResolvedUpstream>,
    /// Maximum allowed request body size in bytes.
    pub request_body_max_bytes: usize,
    /// TCP connection timeout to upstream.
    pub upstream_connect_timeout_ms: u64,
    /// Upstream HTTP request roundtrip timeout.
    pub upstream_request_timeout_ms: u64,
    /// Interval between health check worker polls.
    pub upstream_health_interval_ms: u64,
    /// Health check timeout limit.
    pub upstream_health_timeout_ms: u64,
    /// Telemetry queue capacity.
    pub telemetry_capacity: usize,
    /// Log destination path.
    pub telemetry_log_path: String,
    /// Batch size for event flushes.
    pub telemetry_batch_size: usize,
    /// Max milliseconds interval between telemetry flushes.
    pub telemetry_flush_interval_ms: u64,
}

/// Resolved target upstream instance details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedUpstream {
    /// Unique key name.
    pub id: String,
    /// Provider type (e.g. "ollama", "openai").
    pub provider: String,
    /// Upstream HTTP address.
    pub base_url: String,
    /// Priority sorting key.
    pub priority: u16,
    /// Endpoints targeted for ping checking.
    pub health_path: String,
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
    let upstreams = resolve_upstreams(args, config_file);
    let primary_upstream = upstreams
        .first()
        .expect("resolve_upstreams always returns at least one upstream");

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
        upstream_provider: primary_upstream.provider.clone(),
        upstream_base_url: primary_upstream.base_url.clone(),
        upstreams,
        request_body_max_bytes: args
            .request_body_max_bytes
            .or_else(|| {
                config_file
                    .server
                    .as_ref()
                    .and_then(|s| s.request_body_max_bytes)
            })
            .unwrap_or(10_485_760),
        upstream_connect_timeout_ms: args
            .upstream_connect_timeout_ms
            .or_else(|| {
                config_file
                    .server
                    .as_ref()
                    .and_then(|s| s.upstream_connect_timeout_ms)
            })
            .unwrap_or(5_000),
        upstream_request_timeout_ms: args
            .upstream_request_timeout_ms
            .or_else(|| {
                config_file
                    .server
                    .as_ref()
                    .and_then(|s| s.upstream_request_timeout_ms)
            })
            .unwrap_or(120_000),
        upstream_health_interval_ms: args
            .upstream_health_interval_ms
            .or_else(|| {
                config_file
                    .upstream_health
                    .as_ref()
                    .and_then(|h| h.interval_ms)
            })
            .unwrap_or(5_000),
        upstream_health_timeout_ms: args
            .upstream_health_timeout_ms
            .or_else(|| {
                config_file
                    .upstream_health
                    .as_ref()
                    .and_then(|h| h.timeout_ms)
            })
            .unwrap_or(1_000),
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

fn resolve_upstreams(args: &Args, config_file: &ConfigFile) -> Vec<ResolvedUpstream> {
    let mut candidates = if !config_file.upstreams.is_empty() {
        config_file.upstreams.clone()
    } else if let Some(upstream) = &config_file.upstream {
        vec![upstream.clone()]
    } else {
        vec![UpstreamConfig {
            id: Some("primary".to_string()),
            provider: None,
            base_url: None,
            priority: Some(0),
            health_path: None,
        }]
    };

    if candidates.is_empty() {
        candidates.push(UpstreamConfig {
            id: Some("primary".to_string()),
            provider: None,
            base_url: None,
            priority: Some(0),
            health_path: None,
        });
    }

    let mut resolved = candidates
        .into_iter()
        .enumerate()
        .map(|(index, upstream)| {
            let provider = upstream.provider.unwrap_or_else(|| "mock".to_string());
            let base_url = upstream
                .base_url
                .unwrap_or_else(|| default_base_url(&provider));
            let priority = upstream
                .priority
                .unwrap_or_else(|| index_as_priority(index));
            let health_path = normalize_health_path(
                upstream
                    .health_path
                    .unwrap_or_else(|| default_health_path(&provider)),
            );
            let id = upstream.id.unwrap_or_else(|| {
                if index == 0 {
                    "primary".to_string()
                } else {
                    format!("upstream-{}", index + 1)
                }
            });

            (
                index,
                ResolvedUpstream {
                    id,
                    provider,
                    base_url,
                    priority,
                    health_path,
                },
            )
        })
        .collect::<Vec<_>>();

    resolved.sort_by(|(left_index, left), (right_index, right)| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left_index.cmp(right_index))
    });

    let mut resolved = resolved
        .into_iter()
        .map(|(_, upstream)| upstream)
        .collect::<Vec<_>>();

    if args.upstream_provider.is_some() || args.upstream_base_url.is_some() {
        let primary = resolved
            .first_mut()
            .expect("resolved upstreams always has a primary");

        if let Some(provider) = &args.upstream_provider {
            primary.provider = provider.clone();
            primary.health_path = default_health_path(provider);
            if args.upstream_base_url.is_none() {
                primary.base_url = default_base_url(provider);
            }
        }

        if let Some(base_url) = &args.upstream_base_url {
            primary.base_url = base_url.clone();
        }
    }

    resolved
}

fn default_base_url(provider: &str) -> String {
    match provider {
        "ollama" => "http://127.0.0.1:11434".to_string(),
        _ => "http://127.0.0.1:9000".to_string(),
    }
}

fn default_health_path(provider: &str) -> String {
    match provider {
        "ollama" => "/api/tags".to_string(),
        "vllm" => "/health".to_string(),
        _ => "/healthz".to_string(),
    }
}

fn normalize_health_path(path: String) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    }
}

fn index_as_priority(index: usize) -> u16 {
    u16::try_from(index).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args() -> Args {
        Args {
            config: None,
            host: None,
            port: None,
            upstream_provider: None,
            upstream_base_url: None,
            request_body_max_bytes: None,
            upstream_connect_timeout_ms: None,
            upstream_request_timeout_ms: None,
            upstream_health_interval_ms: None,
            upstream_health_timeout_ms: None,
            telemetry_capacity: None,
            telemetry_log_path: None,
            telemetry_batch_size: None,
            telemetry_flush_interval_ms: None,
        }
    }

    #[test]
    fn test_resolve_config_defaults_mock() {
        let args = default_args();
        let config_file = ConfigFile::default();
        let resolved = resolve_config_values(&args, &config_file);

        assert_eq!(resolved.upstream_provider, "mock");
        assert_eq!(resolved.upstream_base_url, "http://127.0.0.1:9000");
        assert_eq!(
            resolved.upstreams,
            vec![ResolvedUpstream {
                id: "primary".to_string(),
                provider: "mock".to_string(),
                base_url: "http://127.0.0.1:9000".to_string(),
                priority: 0,
                health_path: "/healthz".to_string(),
            }]
        );
        assert_eq!(resolved.upstream_health_interval_ms, 5_000);
        assert_eq!(resolved.upstream_health_timeout_ms, 1_000);
    }

    #[test]
    fn test_resolve_config_defaults_ollama() {
        let args = Args {
            upstream_provider: Some("ollama".to_string()),
            ..default_args()
        };
        let config_file = ConfigFile::default();
        let resolved = resolve_config_values(&args, &config_file);

        assert_eq!(resolved.upstream_provider, "ollama");
        assert_eq!(resolved.upstream_base_url, "http://127.0.0.1:11434");
        assert_eq!(resolved.upstreams[0].health_path, "/api/tags");
    }

    #[test]
    fn test_resolve_config_explicit_base_url_override() {
        let args = Args {
            upstream_provider: Some("ollama".to_string()),
            upstream_base_url: Some("http://custom-ollama:12345".to_string()),
            ..default_args()
        };
        let config_file = ConfigFile::default();
        let resolved = resolve_config_values(&args, &config_file);

        assert_eq!(resolved.upstream_provider, "ollama");
        assert_eq!(resolved.upstream_base_url, "http://custom-ollama:12345");
    }

    #[test]
    fn test_parse_multiple_upstreams_sorted_by_priority() {
        let config_file = toml::from_str::<ConfigFile>(
            r#"
            [[upstreams]]
            id = "secondary"
            provider = "mock"
            base_url = "http://127.0.0.1:9001"
            priority = 20
            health_path = "readyz"

            [[upstreams]]
            id = "primary"
            provider = "ollama"
            base_url = "http://127.0.0.1:11434"
            priority = 10
            "#,
        )
        .expect("multi-upstream TOML should parse");

        let resolved = resolve_config_values(&default_args(), &config_file);

        assert_eq!(resolved.upstream_provider, "ollama");
        assert_eq!(resolved.upstream_base_url, "http://127.0.0.1:11434");
        assert_eq!(
            resolved.upstreams,
            vec![
                ResolvedUpstream {
                    id: "primary".to_string(),
                    provider: "ollama".to_string(),
                    base_url: "http://127.0.0.1:11434".to_string(),
                    priority: 10,
                    health_path: "/api/tags".to_string(),
                },
                ResolvedUpstream {
                    id: "secondary".to_string(),
                    provider: "mock".to_string(),
                    base_url: "http://127.0.0.1:9001".to_string(),
                    priority: 20,
                    health_path: "/readyz".to_string(),
                },
            ]
        );
    }

    #[test]
    fn test_resolve_upstream_health_config() {
        let config_file = toml::from_str::<ConfigFile>(
            r#"
            [upstream_health]
            interval_ms = 250
            timeout_ms = 75
            "#,
        )
        .expect("upstream health TOML should parse");

        let resolved = resolve_config_values(&default_args(), &config_file);

        assert_eq!(resolved.upstream_health_interval_ms, 250);
        assert_eq!(resolved.upstream_health_timeout_ms, 75);
    }

    #[test]
    fn test_example_config_parses() {
        let config_file =
            toml::from_str::<ConfigFile>(include_str!("../examples/config.toml.example"))
                .expect("example config TOML should parse");

        let resolved = resolve_config_values(&default_args(), &config_file);

        assert_eq!(resolved.upstreams.len(), 2);
        assert_eq!(resolved.upstreams[0].health_path, "/healthz");
        assert_eq!(resolved.upstreams[1].health_path, "/api/tags");
        assert_eq!(resolved.upstream_health_interval_ms, 5_000);
        assert_eq!(resolved.upstream_health_timeout_ms, 1_000);
    }

    #[test]
    fn test_legacy_upstream_table_still_parses() {
        let config_file = toml::from_str::<ConfigFile>(
            r#"
            [upstream]
            provider = "ollama"
            base_url = "http://127.0.0.1:11434"
            "#,
        )
        .expect("legacy upstream TOML should parse");

        let resolved = resolve_config_values(&default_args(), &config_file);

        assert_eq!(resolved.upstream_provider, "ollama");
        assert_eq!(resolved.upstream_base_url, "http://127.0.0.1:11434");
        assert_eq!(resolved.upstreams.len(), 1);
        assert_eq!(resolved.upstreams[0].id, "primary");
    }
}
