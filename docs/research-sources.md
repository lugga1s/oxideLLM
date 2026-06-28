# Fontes de Pesquisa

Status: fontes consultadas e recomendadas para agentes  
Data de consolidacao: 2026-06-28

Use fontes primarias sempre que possivel. Se uma fonte mudar, atualize este arquivo e registre ADR quando a mudanca afetar arquitetura.

---

## Agentes, Instrucoes e Multi-Agentes

- OpenAI Codex best practices: https://developers.openai.com/codex/learn/best-practices
- OpenAI Codex AGENTS.md guide: https://developers.openai.com/codex/guides/agents-md
- OpenAI Codex subagents: https://developers.openai.com/codex/subagents
- OpenAI Codex manual: https://developers.openai.com/codex/codex-manual.md
- Anthropic Claude Code memory / CLAUDE.md: https://docs.anthropic.com/en/docs/claude-code/memory
- Anthropic Claude Code best practices: https://code.claude.com/docs/en/best-practices
- Google Gemini CLI: https://github.com/google-gemini/gemini-cli
- DeepSeek coding agents: https://api-docs.deepseek.com/guides/coding_agents
- AGENTS.md community spec: https://agents.md/

Uso no projeto: manter `AGENTS.md`, `GEMINI.md`, `DEEPSEEK.md` e prompts por card como instrucoes curtas, verificaveis e focadas em done-when.

---

## Rust, Async e HTTP

- Tokio: https://tokio.rs/
- Hyper: https://hyper.rs/
- Hyper docs.rs: https://docs.rs/hyper/latest/hyper/
- Axum GitHub: https://github.com/tokio-rs/axum
- Axum docs.rs: https://docs.rs/axum/latest/axum/
- Tower: https://github.com/tower-rs/tower
- bytes crate: https://docs.rs/bytes/latest/bytes/
- Cargo Book: https://doc.rust-lang.org/cargo/
- Cargo check: https://doc.rust-lang.org/cargo/commands/cargo-check.html
- Cargo test: https://doc.rust-lang.org/cargo/commands/cargo-test.html
- Clippy: https://doc.rust-lang.org/clippy/
- rustfmt: https://github.com/rust-lang/rustfmt
- rustup: https://rustup.rs/
- cargo-nextest: https://nexte.st/

## Proxy Rust

- Pingora announcement: https://blog.cloudflare.com/pingora-open-source/
- Pingora GitHub: https://github.com/cloudflare/pingora

Decisao atual: usar Axum/Hyper para MVP e reavaliar Pingora quando a camada L7 exigir recursos mais profundos.

## Streaming e Protocolos LLM

- OpenAI API docs: https://platform.openai.com/docs
- OpenAI streaming responses guide: https://platform.openai.com/docs/guides/streaming-responses
- Anthropic streaming: https://docs.anthropic.com/en/docs/build-with-claude/streaming
- Anthropic Messages API: https://docs.anthropic.com/en/api/messages
- Ollama API: https://docs.ollama.com/api
- Ollama OpenAI compatibility: https://docs.ollama.com/api/openai-compatibility
- MDN Server-Sent Events: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events

## Telemetria e Observabilidade

- tracing crate: https://docs.rs/tracing/latest/tracing/
- tracing-subscriber: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/
- tracing-appender non-blocking: https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/
- OpenTelemetry Rust: https://opentelemetry.io/docs/languages/rust/

## Filas, Buffers e Concorrencia

- crossbeam queue: https://docs.rs/crossbeam-queue/latest/crossbeam_queue/
- Tokio mpsc: https://docs.rs/tokio/latest/tokio/sync/mpsc/
- Rust atomics: https://doc.rust-lang.org/std/sync/atomic/

## Persistencia Analitica

- DuckDB Rust API: https://duckdb.org/docs/api/rust
- DuckDB Parquet: https://duckdb.org/docs/data/parquet/overview
- Apache Arrow Rust: https://arrow.apache.org/rust/
- parquet crate: https://docs.rs/parquet/latest/parquet/
- ClickHouse Rust client: https://github.com/ClickHouse/clickhouse-rs

## Benchmark e Profiling

- Grafana k6 docs: https://grafana.com/docs/k6/latest/
- k6 scenarios: https://grafana.com/docs/k6/latest/using-k6/scenarios/
- k6 thresholds: https://grafana.com/docs/k6/latest/using-k6/thresholds/
- k6 GitHub Action: https://grafana.com/docs/k6/latest/testing-guides/automated-performance-testing/
- wrk GitHub: https://github.com/wg/wrk
- heaptrack: https://github.com/KDE/heaptrack
- DHAT Rust docs: https://docs.rs/dhat/latest/dhat/
- perf Linux: https://perf.wiki.kernel.org/
- perf tutorial: https://perf.wiki.kernel.org/index.php/Tutorial

## GitHub e Automacao

- GitHub Actions: https://docs.github.com/actions
- GitHub Actions workflow syntax: https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions
- Protected branches: https://docs.github.com/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches
- Required status checks: https://docs.github.com/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
- PR templates: https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository
- CODEOWNERS: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners
- GitHub CLI manual: https://cli.github.com/manual/
- `gh pr create`: https://cli.github.com/manual/gh_pr_create
- Gemini CLI GitHub: https://github.com/google-gemini/gemini-cli
- DeepSeek API docs: https://api-docs.deepseek.com/
- Claude Code memory docs: https://docs.anthropic.com/en/docs/claude-code/memory
- GitHub Copilot custom instructions: https://docs.github.com/en/copilot/customizing-copilot/adding-repository-custom-instructions-for-github-copilot
- GitHub Copilot repository instructions: https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions
- OpenAI Codex cloud AGENTS.md: https://developers.openai.com/codex/cloud
- cargo-nextest: https://nexte.st/

## Concorrentes e Referencias de Mercado

- LiteLLM docs: https://docs.litellm.ai/
- LiteLLM DB info: https://docs.litellm.ai/docs/proxy/db_info
- LiteLLM logging callbacks: https://docs.litellm.ai/docs/proxy/logging
- Langfuse docs: https://langfuse.com/docs
- Helicone docs: https://docs.helicone.ai/
- Portkey docs: https://portkey.ai/docs

Uso correto dessas fontes: comparar categorias de arquitetura e validar claims. Nao afirmar comportamento interno de concorrentes sem benchmark ou citacao especifica.
