# Implementation Playbook

Status: plano operacional mestre para construir o produto  
Modo: rapido, pratico, validado por evidencias  
Publico: Codex, Gemini, DeepSeek, agentes auxiliares e engenheiro de contexto

---

## 1. Principio de Execucao

O projeto nao deve ser estrangulado por processo antes de existir. O fluxo correto e:

```text
fazer funcionar
medir
corrigir gargalos reais
documentar a decisao
avancar
```

Gates existem para evitar autoengano, nao para impedir progresso. Um gate amarelo pode ser aceito temporariamente quando o objetivo e destravar aprendizado. Um gate vermelho impede claims publicos, mas nao impede experimentos controlados.

---

## 2. Horizonte do Produto

### MVP funcional

```text
Gateway Rust roda localmente
existe mock SSE
existe benchmark direto vs gateway
gateway responde chat completions em streaming
telemetria minima entra em fila de memoria
```

### Alpha util

```text
gateway proxy real para Ollama ou vLLM
TTFT medido
client disconnect tratado
config basica de upstream
README com resultado local reproduzivel
```

### Beta publica

```text
OpenAI-compatible pass-through robusto
telemetria em micro-batching local
CI estavel
release binario
documentacao de instalacao em menos de 3 minutos
benchmark vLLM direto vs gateway
```

---

## 3. Sequencia de Sessoes

### Sessao 1: Tooling e Compilacao

Objetivo:

```text
instalar Rust/k6 se ausentes e fazer o scaffold compilar
```

Comandos:

```bash
rustc --version
cargo --version
k6 version
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Sucesso:

```text
Cargo compila.
Testes passam.
Clippy passa ou gera lista pequena de correcoes.
```

Se falhar:

```text
corrigir scaffold, nao adicionar feature nova.
```

---

### Sessao 2: Mock SSE e Baseline Direto

Objetivo:

```text
medir o mock direto sem gateway
```

Comandos:

```bash
docker compose up --build mock
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
```

Sucesso:

```text
mock responde SSE com [DONE]
k6 executa sem erro estrutural
RPS/P99 registrados
```

Importante:

```text
este resultado nao prova performance do gateway; ele so cria baseline.
se usar mock Python, benchmark com 1000 VUs pode saturar o mock.
usar mock Rust (mock/src/main.rs) para resultados confiaveis.
```

---

### Sessao 3: Gateway Local Mockado

Objetivo:

```text
rodar gateway Rust e responder SSE pelo proprio gateway
```

Comandos:

```bash
cargo run -- --host 127.0.0.1 --port 8080
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/readyz
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

Sucesso:

```text
gateway sobe
healthz/readyz respondem
k6 recebe [DONE]
telemetry queue nao bloqueia
```

---

### Sessao 4: Proxy Para Mock Upstream

Objetivo:

```text
trocar resposta mockada interna por proxy real para mock externo
```

Mudanca esperada:

```text
cliente -> gateway -> mock SSE externo -> gateway -> cliente
```

Sucesso:

```text
gateway encaminha chunks conforme chegam
nao reconstroi JSON por chunk
client disconnect cancela upstream
```

---

### Sessao 5: Comparacao Direto vs Gateway

Objetivo:

```text
medir overhead real do proxy base
```

Ambiente obrigatorio:

```text
WSL2 ou Linux nativo para testes com 100+ VUs (ver ADR-0007)
Windows aceitavel apenas para validacao funcional com 10 VUs
```

Comandos:

```bash
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

Sucesso verde:

```text
RPS gateway >= 98% do direto
P99 aproximadamente flat
```

Sucesso amarelo aceitavel para MVP:

```text
RPS gateway >= 90% do direto
sem erro funcional
sem crescimento obvio de memoria
```

Se amarelo:

```text
seguir para upstream real se o objetivo for aprendizado;
nao publicar claim de paridade.
```

---

### Sessao 6: Configuracao de Upstream

Objetivo:

```text
adicionar config simples para upstream base_url e provider
```

Entrada minima:

```toml
[server]
host = "127.0.0.1"
port = 8080

[upstream]
provider = "mock"
base_url = "http://127.0.0.1:9000"
```

Sucesso:

```text
gateway nao tem URL hardcoded
README explica variaveis/config
```

---

### Sessao 7: Ollama Local

Objetivo:

```text
usar Ollama OpenAI-compatible como primeiro upstream real local
```

Motivo:

```text
baixo custo
setup local
boa ponte antes de vLLM
```

Sucesso:

```text
POST /v1/chat/completions com stream funciona via gateway
TTFT registrado
erro upstream mapeado
```

---

### Sessao 8: vLLM Native Parity

Objetivo:

```text
comparar vLLM direto vs gateway -> vLLM
```

Sucesso verde:

```text
RPS gateway >= 98% do vLLM direto
P99 aproximadamente flat
TTFT overhead documentado
```

Sucesso amarelo:

```text
RPS gateway entre 90% e 98%
produto funciona
otimizacao vira tarefa propria
```

---

### Sessao 9: Telemetria Final Real

Objetivo:

```text
trocar eventos mockados por accumulator real
```

Campos minimos:

```text
request_id
started_at
completed_at
ttft_ms
bytes_in
bytes_out
status
error_class
```

Sucesso:

```text
evento final entra na fila
fila cheia nao bloqueia cliente
drops sao contados
```

---

### Sessao 10: Micro-batching Local Simples

Objetivo:

```text
persistir em JSONL por lote antes de Parquet/DuckDB
```

Por que JSONL primeiro:

```text
mais simples
facil de debugar
destrava validacao de desacoplamento
Parquet/DuckDB entram depois
```

Sucesso:

```text
logs aparecem em blocos
cliente nao espera flush
flush por tempo ou tamanho
```

---

### Sessao 11: Profiling Basico

Objetivo:

```text
achar gargalos reais antes de otimizar
```

Ambiente obrigatorio:

```text
WSL2 ou Linux nativo (ver ADR-0007)
perf e heaptrack nao estao disponiveis em Windows
```

Comandos Linux:

```bash
perf stat -p <PID>
heaptrack ./target/release/oxidellm
```

Sucesso:

```text
hotspots conhecidos
alocacoes suspeitas listadas
issue criada por gargalo
```

---

### Sessao 12: GitHub Alpha

Objetivo:

```text
preparar repo para publico alpha
```

Checklist:

```text
README com quickstart
CI verde
licenca correta
benchmark real marcado como local
issues templates
roadmap claro
```

---

## 4. Ordem Que Nao Deve Ser Invertida

Nao fazer antes do primeiro benchmark real:

- dashboard;
- 100 provedores;
- autenticacao enterprise complexa;
- ClickHouse obrigatorio;
- Kubernetes;
- cache sem baseline;
- claims publicos agressivos.

Fazer primeiro:

- compilar;
- streamar;
- medir;
- comparar;
- conectar upstream real.

---

## 5. Como Usar Com Outros Agentes

Para cada sessao:

1. escolha um task card em `docs/agent-task-cards.md`;
2. cole o prompt correspondente de `docs/agent-prompts.md`;
3. exija o formato de resposta de `docs/multi-agent-handoff.md`;
4. aceite resultado amarelo somente se o objetivo for aprendizado;
5. nao aceite claim verde sem comando e resultado.

---

## 6. Regra de Ouro do MVP

Se uma mudanca nao ajuda a chegar em:

```text
mock -> gateway -> k6 -> upstream real -> telemetria minima
```

ela provavelmente deve esperar.
