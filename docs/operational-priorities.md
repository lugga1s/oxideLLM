# Operational Priorities

Status: mapa de foco para nao perder velocidade

---

## 1. Prioridade Absoluta

```text
produto funcional local com benchmark basico
```

Tudo que nao ajuda isso deve esperar.

---

## 2. Fazer Agora

- instalar toolchain;
- compilar;
- rodar gateway;
- validar mock SSE;
- rodar k6 direto;
- rodar k6 via gateway;
- conectar mock externo;
- conectar Ollama/vLLM;
- medir TTFT;
- publicar resultado local honesto.

---

## 3. Fazer Depois

- Parquet;
- DuckDB;
- ClickHouse;
- HTTP/3;
- dashboard;
- 100 provedores;
- plugins;
- auth enterprise;
- rate limit sofisticado;
- Kubernetes manifests;
- Helm chart;
- Wall of Fame.

---

## 4. Nao Fazer Por Enquanto

- trocar Rust por Go;
- migrar para Pingora antes de medir Axum/Hyper;
- adicionar Postgres no hot path;
- implementar Anthropic completo antes de OpenAI-compatible passar;
- otimizar sem profiling;
- prometer "mais rapido que LiteLLM" sem benchmark publico;
- criar processo pesado de compliance antes do alpha.

---

## 5. Quando Aceitar Imperfeicao

Aceite temporariamente:

- JSONL antes de Parquet;
- config simples antes de config enterprise;
- mock antes de vLLM;
- benchmark local antes de laboratorio;
- fila MPSC pronta antes de ring buffer custom;
- Axum antes de Pingora.

Nao aceite:

- bloquear cliente por disco;
- fila unbounded;
- segredo em log;
- claim sem dado;
- stream quebrado.

---

## 6. Frase Guia

```text
O produto precisa respirar antes de correr maratona.
```
