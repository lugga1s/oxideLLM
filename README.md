# oxideLLM

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

Este repo esta em preparacao para alpha v1 funcional:

- gateway Rust com endpoints `/healthz`, `/readyz` e `/v1/chat/completions`;
- configuracao por CLI, variaveis de ambiente e TOML;
- proxy para upstream OpenAI-compatible/Ollama/Groq em fluxo SSE;
- mock SSE em Rust para benchmark local;
- telemetria final em fila bounded e worker de micro-batching JSONL;
- benchmarks locais existentes reconciliados em `benchmarks/alpha-v1-benchmark-summary.md`.

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

O endpoint de chat encaminha o corpo da requisicao para o upstream configurado e retransmite a resposta SSE. Para validacao local sem provedor real, suba o mock Rust em `mock/` na porta `9000`.

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

O projeto usa k6 para comparar a eficiencia do gateway frente a conexao direta com o upstream ou mock equivalente.

O resumo oficial dos artefatos existentes esta em `benchmarks/alpha-v1-benchmark-summary.md`. A leitura correta hoje e conservadora: os artefatos brutos antigos registram RPS, P95 e taxa de erro, mas nao registram P99. Por isso, estes numeros servem para reconciliar o estado do alpha; um claim publico completo de P99 exige uma nova execucao com o `handleSummary()` atual.

Depois do ajuste do script k6, tambem existe um smoke WSL2 curto com P95/P99 no resumo oficial. Ele valida a geracao do artefato JSON, mas nao substitui o gate de alta concorrencia.

### Resultados Reconciliados do Stage 2

Fonte: artefatos locais em `benchmarks/results/` gerados antes desta passada. Esse diretorio e ignorado por Git; o resumo versionavel fica em `benchmarks/alpha-v1-benchmark-summary.md`.

| Caminho de execucao | RPS medio | Latencia P95 | P99 no JSON | HTTP erros | Leitura |
|---|---:|---:|---|---:|---|
| Conexao direta ao mock Rust | 20.282,05 req/s | 59,64 ms | nao registrado | 0,00% | baseline |
| Gateway com telemetria ativa | 17.831,39 req/s | 74,40 ms | nao registrado | 0,00% | 12,08% de degradacao vs direto |
| Gateway com logs em `/dev/null` | 18.014,34 req/s | 73,68 ms | nao registrado | 0,00% | 11,18% de degradacao vs direto |

O numero de 1,01% nao deve ser lido como overhead do gateway contra o direto. Pelos artefatos visiveis, ele representa apenas a diferenca de RPS entre o gateway com telemetria ativa e o gateway com logs em `/dev/null`.

Comandos para executar os testes locais:

```bash
# 1. Subir mock
cd mock && cargo run --release

# 2. Subir gateway
cargo run --release

# 3. Rodar k6 direto vs gateway com resumo JSON
k6 run -e RUN_LABEL=stage-02-direct \
  -e TARGET_URL=http://localhost:9000/v1/chat/completions \
  -e SUMMARY_PATH=benchmarks/results/stage-02-direct-summary.json \
  k6/proxy-vs-direct.js

k6 run -e RUN_LABEL=stage-02-gateway \
  -e TARGET_URL=http://localhost:8080/v1/chat/completions \
  -e SUMMARY_PATH=benchmarks/results/stage-02-gateway-summary.json \
  k6/proxy-vs-direct.js
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

Voce pode validar a integracao funcional com o provedor real local **Ollama** executando um modelo local (como `tinyllama` ou `llama3`).

1. Garanta que o Ollama esteja instalado e rodando em sua maquina:
   ```bash
   ollama pull tinyllama
   ```

2. Suba o gateway apontando para o Ollama como upstream:
   ```bash
   cargo run --release -- --upstream-provider ollama
   ```
   *(Nota: O gateway ira associar automaticamente a URL padrao do Ollama: `http://127.0.0.1:11434`)*

3. Faca uma requisicao de chat streaming via gateway para testar a resposta:
   ```bash
   curl -N -X POST http://127.0.0.1:8080/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{
       "model": "tinyllama",
       "messages": [{"role": "user", "content": "Ola, voce e o tinyllama?"}],
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

## Apoio & Doacoes (Sponsors)

O **oxideLLM** e um projeto open-source gratuito para uso local. A nossa estrategia de monetizacao e voluntaria e baseada no valor economizado (reducao de TCO de infraestrutura). Se o gateway economiza CPU, memoria e custos de API para a sua organizacao, considere apoiar o projeto:

- [GitHub Sponsors](https://github.com/sponsors/lugga1s)
- [Open Collective](https://opencollective.com/oxidellm) (planejado)
- [Buy Me a Coffee](https://www.buymeacoffee.com/lugga1s) (planejado)

### Nossos Patrocinadores (Sponsors)

Abaixo estao listados os doadores que apoiam ativamente o projeto (atualizado de forma automatizada):

<!-- sponsors-start -->
*(Ainda nao ha doacoes registradas. Seja o primeiro a apoiar o projeto!)*
<!-- sponsors-end -->

## Licenca

Este projeto usa **AGPL-3.0-or-later**. A escolha protege o projeto contra forks SaaS fechados: se alguem modificar o gateway e oferecer o software como servico de rede, deve disponibilizar o codigo-fonte correspondente dessas modificacoes conforme os termos da AGPL.

Antes do primeiro release publico, a estrategia de licenca deve ser revisada em [licensing-strategy.md](<docs/licensing-strategy.md>).
