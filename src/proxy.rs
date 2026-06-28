// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proxy routing and request forward logic for oxideLLM.

use crate::config::ResolvedUpstream;
use crate::drain::UpstreamHealthState;
use axum::http::HeaderName;

/// Returns upstream indices ordered for a single request.
///
/// The input slice is already sorted by configured priority. When a model name
/// carries an obvious provider hint, matching upstreams are moved to the front
/// while preserving relative priority inside the matching and non-matching
/// groups. The function does not inspect health; callers can use the returned
/// indices to try only currently healthy upstreams.
pub fn ordered_upstream_indices(upstreams: &[ResolvedUpstream], model: Option<&str>) -> Vec<usize> {
    let indices = 0..upstreams.len();

    let Some(model) = model.filter(|name| !name.trim().is_empty()) else {
        return indices.collect();
    };

    let (mut matching, non_matching): (Vec<_>, Vec<_>) =
        indices.partition(|index| model_matches_upstream(model, &upstreams[*index]));
    matching.extend(non_matching);
    matching
}

/// Selects the best healthy upstream for a request.
///
/// This is a convenience wrapper over [`ordered_upstream_indices`] for callers
/// that only need the first eligible target instead of the full failover order.
pub fn select_upstream(
    upstreams: &[ResolvedUpstream],
    health: &UpstreamHealthState,
    model: Option<&str>,
) -> Option<ResolvedUpstream> {
    if upstreams.is_empty() {
        return None;
    }

    for index in ordered_upstream_indices(upstreams, model) {
        if health.is_healthy(index) {
            return Some(upstreams[index].clone());
        }
    }

    None
}

/// Determines whether a client request header should be forwarded upstream.
///
/// Hop-by-hop headers are stripped because they describe only the current HTTP
/// connection and can corrupt proxy behavior if forwarded to a different
/// connection. End-to-end headers such as `authorization`, `content-type`, and
/// tracing headers remain eligible for forwarding.
pub fn should_forward_header(name: &HeaderName) -> bool {
    !is_hop_by_hop_header(name)
}

/// Determines whether an upstream response header should be sent to the client.
///
/// The same hop-by-hop filtering rule applies in both proxy directions.
pub fn should_forward_response_header(name: &HeaderName) -> bool {
    !is_hop_by_hop_header(name)
}

/// Determines whether a response status allows trying the next upstream.
///
/// The list is intentionally narrow: 429 and transient gateway errors are safe
/// failover candidates, while 4xx request errors are returned to the caller.
pub fn should_try_next_upstream(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 502 | 503 | 504)
}

/// Formats the OpenAI-compatible chat completions endpoint URL for an upstream.
pub fn chat_completions_url(upstream: &ResolvedUpstream) -> String {
    format!(
        "{}/v1/chat/completions",
        upstream.base_url.trim_end_matches('/')
    )
}

fn model_matches_upstream(model: &str, upstream: &ResolvedUpstream) -> bool {
    let model_is_gpt = contains_ascii_case_insensitive(model, "gpt")
        || contains_ascii_case_insensitive(model, "openai");
    let model_is_llama = contains_ascii_case_insensitive(model, "llama")
        || contains_ascii_case_insensitive(model, "ollama");
    let model_is_claude = contains_ascii_case_insensitive(model, "claude")
        || contains_ascii_case_insensitive(model, "anthropic");

    let upstream_is_openai = contains_ascii_case_insensitive(&upstream.provider, "openai")
        || contains_ascii_case_insensitive(&upstream.id, "openai");
    let upstream_is_ollama = contains_ascii_case_insensitive(&upstream.provider, "ollama")
        || contains_ascii_case_insensitive(&upstream.id, "ollama");
    let upstream_is_anthropic = contains_ascii_case_insensitive(&upstream.provider, "anthropic")
        || contains_ascii_case_insensitive(&upstream.id, "anthropic");

    (model_is_gpt && upstream_is_openai)
        || (model_is_llama && upstream_is_ollama)
        || (model_is_claude && upstream_is_anthropic)
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();

    if needle.is_empty() {
        return true;
    }

    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn is_hop_by_hop_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "host"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
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
    fn test_request_routing_by_model_is_case_insensitive() {
        let upstreams = vec![
            ResolvedUpstream {
                id: "openai-provider".to_string(),
                provider: "openai".to_string(),
                base_url: "http://localhost:8001".to_string(),
                priority: 10,
                health_path: "/healthz".to_string(),
            },
            ResolvedUpstream {
                id: "local-ollama".to_string(),
                provider: "Ollama".to_string(),
                base_url: "http://localhost:8002".to_string(),
                priority: 20,
                health_path: "/healthz".to_string(),
            },
        ];
        let health = UpstreamHealthState::new(2);

        let selected = select_upstream(&upstreams, &health, Some("LLaMA3-8B"));

        assert_eq!(selected.unwrap().id, "local-ollama");
    }

    #[test]
    fn test_ordered_upstream_indices_prioritizes_all_model_matches() {
        let upstreams = vec![
            ResolvedUpstream {
                id: "primary-openai".to_string(),
                provider: "openai".to_string(),
                base_url: "http://localhost:8001".to_string(),
                priority: 10,
                health_path: "/healthz".to_string(),
            },
            ResolvedUpstream {
                id: "fallback-ollama-a".to_string(),
                provider: "ollama".to_string(),
                base_url: "http://localhost:8002".to_string(),
                priority: 20,
                health_path: "/healthz".to_string(),
            },
            ResolvedUpstream {
                id: "fallback-ollama-b".to_string(),
                provider: "ollama".to_string(),
                base_url: "http://localhost:8003".to_string(),
                priority: 30,
                health_path: "/healthz".to_string(),
            },
        ];

        let order = ordered_upstream_indices(&upstreams, Some("llama3"));

        assert_eq!(order, vec![1, 2, 0]);
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
        assert!(!should_forward_header(&HeaderName::from_static(
            "keep-alive"
        )));
        assert!(!should_forward_header(&HeaderName::from_static(
            "proxy-authorization"
        )));
        assert!(!should_forward_header(&HeaderName::from_static("te")));
        assert!(!should_forward_header(&HeaderName::from_static("trailer")));
        assert!(!should_forward_header(&HeaderName::from_static("upgrade")));
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
