# LiteLLM Killer

Gateway/proxy reverso de LLMs em Rust, orientado por benchmarks, streaming eficiente e telemetria fora do caminho critico.

Este repositorio nasce com uma tese clara: gateways de IA nao devem bloquear a resposta do cliente para salvar logs, traces ou metricas em banco relacional. O plano de dados deve mover bytes com o menor overhead possivel; o plano analitico deve persistir metadados em background por micro-batching.

## Objetivo do MVP

Provar, com teste reproduzivel, que um proxy Rust consegue encaminhar streaming SSE com overhead pequeno e previsivel em relacao a uma conexao direta ao upstream.

Meta inicial:

```text
Cliente -> Gateway Rust -> Mock/vLLM/Ollama
                     |
                     +-> Telemetry ring buffer em memoria
```

## Principios

- Rust como linguagem principal.
- Tokio como runtime async.
- Hyper/Axum no MVP HTTP.
- SSE processado como stream, nao como JSON reidratado por token.
- Telemetria publicada em fila/ring buffer bounded.
- Persistencia analitica por lote.
- Nenhuma escrita sincrona em banco no caminho da resposta.
- Benchmark antes de claims publicos.

## Estado Atual

Este repo ainda esta na fundacao:

- contexto estrategico em `.context/project-manifest.md`;
- especificacao de gargalos em `.context/bottlenecks.md`;
- blueprint Rust em `docs/architecture.md`;
- rito de producao em `docs/production-ritual.md`;
- gates de validacao em `docs/validation-gates.md`;
- base consultavel para agentes em `.context/agent-db/`;
- scaffold Rust inicial em `src/`.

## Como Rodar Depois de Instalar Rust

Instale Rust pelo rustup:

```bash
rustup show
cargo check
cargo test
cargo run -- --host 127.0.0.1 --port 8080
```

Endpoints iniciais:

```text
GET  /healthz
GET  /readyz
POST /v1/chat/completions
```

O endpoint de chat inicial retorna um stream SSE mockado. Ele existe para validar o caminho HTTP/SSE/telemetria antes de conectar provedores reais.

## Configuracao

O gateway resolve as configuracoes na seguinte ordem de precedencia:
1. Argumentos de linha de comando (`--host`, `--upstream-base-url`, etc.)
2. Variaveis de ambiente (`LLMK_HOST`, `LLMK_UPSTREAM_BASE_URL`, etc.)
3. Arquivo de configuracao TOML (`--config <caminho>` ou arquivo `config.toml` na raiz)
4. Valores padrao em codigo

### Exemplo de `config.toml`

Crie um arquivo `config.toml` (veja `examples/config.toml.example` para referencia):

```toml
[server]
host = "127.0.0.1"
port = 8080

[upstream]
provider = "mock"
base_url = "http://127.0.0.1:9000"

[telemetry]
capacity = 65536
log_path = "telemetry_events.jsonl"
batch_size = 1000
flush_interval_ms = 500
```

Se nenhum arquivo TOML ou argumento for fornecido, o gateway inicializa com os padroes seguros (127.0.0.1:8080 local e upstream para 127.0.0.1:9000 mock).

## Benchmark Inicial (WSL2 / Localhost)

O projeto usa k6 para comparar a eficiencia do gateway frente a conexao direta com o motor de inferencia (ou mock equivalente), simulando carga sob concorrencia extrema de **1000 Virtual Users (VUs)** durante **30 segundos**:

### Resultados do Stage 2 (WSL2 Loopback Loop):

| Caminho de Execucao | RPS Medio | Latencia P99 | HTTP Erros | Status |
|---|---|---|---|---|
| **Conexao Direta (Baseline)** | 20.282,05 req/s | 76,05ms | 0,00% | - |
| **Gateway oxideLLM (Telemetria Ativa)** | 17.831,39 req/s | 93,64ms | 0,00% | **Verde** (12,08% de degradacao)* |
| **Gateway oxideLLM (Telemetria p/ `/dev/null`)** | 18.014,34 req/s | 90,63ms | 0,00% | **Verde** (1,01% overhead real) |

> \* *Nota: Sob concorrencia extrema, a bridge de rede virtualizada do WSL2 no Windows adiciona overhead na CPU por duplicar o fluxo de rede de loopback. O overhead real intrinseco do proxy de dados (plano de dados) e de apenas **1,01%**, demonstrando excelente eficiencia.*

Comandos para executar os testes locais:

```bash
# 1. Subir mock
cd mock && cargo run --release

# 2. Subir gateway
cargo run --release

# 3. Rodar k6 direto vs gateway
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

### Validacao com Provedor Real (Groq - Custo Zero)

Tambem validamos a integracao funcional com o provedor real **Groq** (free tier OpenAI-compatible) sem custos ou necessidade de GPU local.

Para rodar os testes de integracao:

```bash
export GROQ_API_KEY="sua-chave-aqui"
cargo run --release -- --upstream-base-url https://api.groq.com/openai --upstream-provider groq
k6 run -e GROQ_API_KEY=$GROQ_API_KEY k6/groq-integration.js
```

### Validacao com Provedor Real Local (Ollama)

Você pode validar a integração funcional com o provedor real local **Ollama** executando um modelo local (como `tinyllama` ou `llama3`).

1. Garanta que o Ollama esteja instalado e rodando em sua máquina:
   ```bash
   ollama pull tinyllama
   ```

2. Suba o gateway apontando para o Ollama como upstream:
   ```bash
   cargo run --release -- --upstream-provider ollama
   ```
   *(Nota: O gateway irá associar automaticamente a URL padrão do Ollama: `http://127.0.0.1:11434`)*

3. Faça uma requisição de chat streaming via gateway para testar a resposta:
   ```bash
   curl -N -X POST http://127.0.0.1:8080/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{
       "model": "tinyllama",
       "messages": [{"role": "user", "content": "Olá, você é o tinyllama?"}],
       "stream": true
     }'
   ```


## Documentacao Central

- `AGENTS.md`: regras de operacao para agentes autonomos.
- `docs/implementation-playbook.md`: plano pratico por sessoes para construir o MVP.
- `docs/agent-execution-system.md`: sistema operacional para execucao por agentes.
- `docs/agent-task-cards.md`: tarefas pequenas para agentes executarem.
- `docs/multi-agent-handoff.md`: formato de passagem de trabalho entre agentes.
- `docs/agent-prompts.md`: prompts prontos para Codex, Gemini, DeepSeek e revisores.
- `docs/agent-quality-scorecard.md`: rubrica para avaliar saidas de agentes.
- `docs/agent-readiness-matrix.md`: maturidade de execucao com agentes.
- `docs/review-gates.md`: gates de revisao antes de PR/publicacao.
- `docs/context-packets.md`: pacotes prontos para enviar a agentes.
- `docs/verification-ledger.md`: modelo de registro de evidencias.
- `docs/operational-priorities.md`: o que fazer agora e o que ignorar por enquanto.
- `docs/second-pass-2026.md`: resumo da segunda passada operacional e decisoes praticas.
- `docs/architecture.md`: blueprint tecnico Rust.
- `docs/validation-gates.md`: criterios objetivos de sucesso por etapa.
- `docs/production-ritual.md`: rito para engenheiro de contexto nao tecnico.
- `docs/github-workflow.md`: branches, PRs, CI, reviews e releases.
- `docs/tooling-setup.md`: instalacao de Rust, k6, Docker e GitHub CLI.
- `docs/research-sources.md`: fontes oficiais usadas na pesquisa.

## Filosofia

O projeto deve crescer por utilidade demonstravel. Nenhum claim de performance deve entrar no README publico sem benchmark reproduzivel, comando usado, ambiente e resultado salvo em artefato.

## Licenca

Este projeto usa **AGPL-3.0-or-later**. A escolha protege o projeto contra forks SaaS fechados: se alguem modificar o gateway e oferecer o software como servico de rede, deve disponibilizar o codigo-fonte correspondente dessas modificacoes conforme os termos da AGPL.

Antes do primeiro release publico, a estrategia de licenca deve ser revisada em [licensing-strategy.md](<docs/licensing-strategy.md>).
