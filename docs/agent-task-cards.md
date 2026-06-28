# Agent Task Cards

Status: tarefas prontas para Codex, Gemini, DeepSeek e agentes auxiliares

Cada card tem escopo pequeno. Um agente deve pegar um card, executar, validar e devolver handoff.

---

## TC-001: Verificar Tooling Local

Objetivo:

```text
confirmar Rust, k6, Docker, Git e gh
```

Arquivos permitidos:

```text
docs/tooling-setup.md
```

Comandos:

```bash
rustc --version
cargo --version
k6 version
docker version
git --version
gh --version
```

Sucesso:

```text
lista clara de ferramentas disponiveis/ausentes
```

---

## TC-002: Fazer Scaffold Rust Compilar

Objetivo:

```text
fazer cargo check/test/clippy passarem
```

Arquivos permitidos:

```text
Cargo.toml
src/**
```

Comandos:

```bash
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Sucesso:

```text
todos comandos passam
```

Proibido:

```text
adicionar feature nova antes de compilar
```

---

## TC-003: Validar Mock SSE

Objetivo:

```text
subir mock e confirmar stream [DONE]
```

Arquivos permitidos:

```text
mock/**
docker-compose.yml
k6/**
```

Comandos:

```bash
docker compose up --build mock
curl -N -X POST http://localhost:9000/v1/chat/completions -H "Content-Type: application/json" -d "{}"
```

Sucesso:

```text
resposta contem data: [DONE]
```

---

## TC-004: Rodar Baseline k6 Direto

Objetivo:

```text
medir mock direto
```

Arquivos permitidos:

```text
k6/**
benchmarks/**
```

Comandos:

```bash
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
```

Sucesso:

```text
RPS, P95, P99 e taxa de erro registrados
```

---

## TC-005: Gateway Health e Ready

Objetivo:

```text
garantir que gateway sobe e endpoints basicos respondem
```

Arquivos permitidos:

```text
src/**
Cargo.toml
```

Comandos:

```bash
cargo run -- --host 127.0.0.1 --port 8080
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/readyz
```

Sucesso:

```text
status ok/ready
```

---

## TC-006: Gateway SSE Mockado

Objetivo:

```text
confirmar /v1/chat/completions em SSE
```

Arquivos permitidos:

```text
src/**
k6/**
```

Comandos:

```bash
curl -N -X POST http://127.0.0.1:8080/v1/chat/completions -H "Content-Type: application/json" -d "{}"
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

Sucesso:

```text
stream contem [DONE]
k6 passa sem erro estrutural
```

---

## TC-007: Proxy Para Mock Upstream

Objetivo:

```text
gateway encaminha SSE de mock externo
```

Arquivos permitidos:

```text
src/**
docs/protocol-contracts.md
```

Sucesso:

```text
cliente recebe chunks do mock via gateway
```

Proibido:

```text
parsear JSON completo por chunk
```

---

## TC-008: Config Minima

Objetivo:

```text
adicionar configuracao de server/upstream
```

Arquivos permitidos:

```text
src/**
examples/**
README.md
```

Sucesso:

```text
base_url do upstream configuravel
```

---

## TC-009: TTFT Accumulator

Objetivo:

```text
medir time to first token
```

Arquivos permitidos:

```text
src/**
docs/telemetry-schema.md
```

Sucesso:

```text
ttft_ms aparece no evento final
```

---

## TC-010: Telemetry Queue Overflow

Objetivo:

```text
validar fila cheia sem bloquear cliente
```

Arquivos permitidos:

```text
src/telemetry.rs
tests/**
```

Sucesso:

```text
teste cobre overflow
drops sao contados
```

---

## TC-011: Micro-batching JSONL

Objetivo:

```text
persistir eventos em JSONL por lote
```

Arquivos permitidos:

```text
src/**
docs/telemetry-schema.md
```

Sucesso:

```text
flush por tempo/tamanho
cliente nao espera disco
```

---

## TC-012: Ollama Upstream

Objetivo:

```text
usar Ollama OpenAI-compatible como upstream real
```

Arquivos permitidos:

```text
src/**
examples/**
README.md
docs/protocol-contracts.md
```

Sucesso:

```text
stream real via gateway
```

---

## TC-013: vLLM Parity Runbook

Objetivo:

```text
documentar e rodar comparacao vLLM direto vs gateway
```

Arquivos permitidos:

```text
benchmarks/**
docs/validation-gates.md
```

Sucesso:

```text
resultado direto e gateway registrados
```

---

## TC-014: GitHub PR Draft

Objetivo:

```text
criar branch e PR draft com validacao
```

Arquivos permitidos:

```text
.github/**
docs/github-workflow.md
```

Comandos:

```bash
git checkout -b feature/<nome>
git push -u origin feature/<nome>
gh pr create --draft
```

Sucesso:

```text
PR draft criado ou bloqueio de auth/remoto reportado
```

---

## TC-015: Performance Review

Objetivo:

```text
analisar se uma mudanca violou o hot path
```

Arquivos permitidos:

```text
todos para leitura
somente docs se precisar registrar findings
```

Sucesso:

```text
lista de riscos concretos com arquivo/linha
```

---

## TC-016: Docs Consistency Pass

Objetivo:

```text
garantir que README, AGENTS e docs concordam
```

Arquivos permitidos:

```text
README.md
AGENTS.md
docs/**
.context/**
```

Sucesso:

```text
sem contradicao sobre stack, licenca, gates ou ordem de execucao
```

---

## TC-017: Context Validation Script

Objetivo:

```text
rodar validacao automatica de contexto antes de handoff
```

Arquivos permitidos:

```text
scripts/validate_context.ps1
docs/**
.context/**
```

Comandos:

```powershell
.\scripts\validate_context.ps1
```

Sucesso:

```text
script passa
JSONs validos
arquivos criticos existem
sem caracteres fora de ASCII nos documentos principais
```
