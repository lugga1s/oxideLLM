// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proxy routing and request forward logic for oxideLLM.

#![allow(dead_code)]

use crate::config::ResolvedUpstream;
use crate::drain::UpstreamHealthState;
use axum::http::HeaderName;

/// Selects the best upstream based on health status and optional model name routing.
pub fn select_upstream(
    upstreams: &[ResolvedUpstream],
    health: &UpstreamHealthState,
    model: Option<&str>,
) -> Option<ResolvedUpstream> {
    if upstreams.is_empty() {
        return None;
    }

    // 1. Model-based routing if a model is provided
    if let Some(m) = model {
        let is_gpt = m.contains("gpt") || m.contains("openai");
        let is_llama = m.contains("llama") || m.contains("ollama");
        let is_claude = m.contains("claude") || m.contains("anthropic");

        let mut matched_upstream = None;

        for (index, upstream) in upstreams.iter().enumerate() {
            if !health.is_healthy(index) {
                continue;
            }

            let matches_gpt =
                is_gpt && (upstream.provider.contains("openai") || upstream.id.contains("openai"));
            let matches_llama = is_llama
                && (upstream.provider.contains("ollama") || upstream.id.contains("ollama"));
            let matches_claude = is_claude
                && (upstream.provider.contains("anthropic") || upstream.id.contains("anthropic"));

            if matches_gpt || matches_llama || matches_claude {
                matched_upstream = Some(upstream.clone());
                break;
            }
        }

        if let Some(u) = matched_upstream {
            return Some(u);
        }
    }

    // 2. Default: select the first healthy upstream in order of priority (already sorted)
    for (index, upstream) in upstreams.iter().enumerate() {
        if health.is_healthy(index) {
            return Some(upstream.clone());
        }
    }

    None
}

/// Determines whether a HTTP header should be forwarded from client to upstream.
pub fn should_forward_header(name: &HeaderName) -> bool {
    name != axum::http::header::HOST
        && name != axum::http::header::CONNECTION
        && name != axum::http::header::TRANSFER_ENCODING
}

/// Determines whether the proxy should attempt to failover to the next upstream based on response status.
pub fn should_try_next_upstream(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 502 | 503 | 504)
}

/// Formats the chat completions endpoint URL for the upstream.
pub fn chat_completions_url(upstream: &ResolvedUpstream) -> String {
    format!(
        "{}/v1/chat/completions",
        upstream.base_url.trim_end_matches('/')
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use std::time::Duration;

    #[test]
    fn test_provider_selection_prefers_healthy() {
        let upstreams = vec![
            ResolvedUpstream {
                id: "primary".to_string(),
                provider: "openai".to_string(),
                base_url: "http://localhost:8001".to_string(),
                priority: 10,
                health_path: "/healthz".to_string(),
            },
            ResolvedUpstream {
                id: "secondary".to_string(),
                provider: "ollama".to_string(),
                base_url: "http://localhost:8002".to_string(),
                priority: 20,
                health_path: "/healthz".to_string(),
            },
        ];
        let health = UpstreamHealthState::new(2);

        // Both healthy -> prefers primary (index 0) due to priority/ordering
        let selected = select_upstream(&upstreams, &health, None);
        assert_eq!(selected.unwrap().id, "primary");
    }

    #[test]
    fn test_failover_on_unhealthy_provider() {
        let upstreams = vec![
            ResolvedUpstream {
                id: "primary".to_string(),
                provider: "openai".to_string(),
                base_url: "http://localhost:8001".to_string(),
                priority: 10,
                health_path: "/healthz".to_string(),
            },
            ResolvedUpstream {
                id: "secondary".to_string(),
                provider: "ollama".to_string(),
                base_url: "http://localhost:8002".to_string(),
                priority: 20,
                health_path: "/healthz".to_string(),
            },
        ];
        let health = UpstreamHealthState::new(2);
        health.set_healthy(0, false); // Mark primary unhealthy

        // Primary unhealthy -> falls back to secondary
        let selected = select_upstream(&upstreams, &health, None);
        assert_eq!(selected.unwrap().id, "secondary");
    }

    #[test]
    fn test_request_routing_by_model() {
        let upstreams = vec![
            ResolvedUpstream {
                id: "openai-provider".to_string(),
                provider: "openai".to_string(),
                base_url: "http://localhost:8001".to_string(),
                priority: 10,
                health_path: "/healthz".to_string(),
            },
            ResolvedUpstream {
                id: "ollama-provider".to_string(),
                provider: "ollama".to_string(),
                base_url: "http://localhost:8002".to_string(),
                priority: 20,
                health_path: "/healthz".to_string(),
            },
        ];
        let health = UpstreamHealthState::new(2);

        // Model is llama -> routes to ollama-provider even though openai-provider has higher priority
        let selected = select_upstream(&upstreams, &health, Some("llama3-8b"));
        assert_eq!(selected.unwrap().id, "ollama-provider");

        // Model is gpt -> routes to openai-provider
        let selected_gpt = select_upstream(&upstreams, &health, Some("gpt-4o"));
        assert_eq!(selected_gpt.unwrap().id, "openai-provider");
    }

    #[test]
    fn test_proxy_returns_error_on_all_providers_down() {
        let upstreams = vec![ResolvedUpstream {
            id: "primary".to_string(),
            provider: "openai".to_string(),
            base_url: "http://localhost:8001".to_string(),
            priority: 10,
            health_path: "/healthz".to_string(),
        }];
        let health = UpstreamHealthState::new(1);
        health.set_healthy(0, false); // Mark all unhealthy

        let selected = select_upstream(&upstreams, &health, None);
        assert!(selected.is_none());
    }

    #[test]
    fn test_request_headers_forwarded_correctly() {
        assert!(should_forward_header(&HeaderName::from_static(
            "authorization"
        )));
        assert!(should_forward_header(&HeaderName::from_static(
            "content-type"
        )));
        assert!(!should_forward_header(&HeaderName::from_static("host")));
        assert!(!should_forward_header(&HeaderName::from_static(
            "connection"
        )));
    }

    #[tokio::test]
    async fn test_timeout_enforcement() {
        // Spawn a slow mock server
        let app = axum::Router::new().route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(150)).await;
                "ok"
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Create a client with a 50ms timeout
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(50))
            .build()
            .unwrap();

        let res = client.get(format!("http://{}/slow", addr)).send().await;
        assert!(res.is_err());
        assert!(res.unwrap_err().is_timeout());

        handle.abort();
    }
}
