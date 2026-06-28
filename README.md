# oxideLLM

O gateway leve que organiza o trafego entre seu aplicativo e multiplos provedores de IA (OpenAI, Ollama, vLLM, Groq, etc.).

Seu aplicativo precisa de inteligencia artificial, mas voce:
* **Depende de um unico provedor** e tem medo de downtime/quedas?
* **Quer trocar de modelo** de IA sem reescrever o seu codigo?
* **Precisa medir latencia e custos** de cada chamada em tempo real?

**oxideLLM** resolve isso de forma simples com:
* **Uma unica API compativel com OpenAI**: Integre uma vez, use qualquer provedor suportado.
* **Roteamento inteligente e fallback automatico**: Se o provedor principal falhar, o gateway desvia a requisicao para um upstream ativo de forma transparente (failover dinamico).
* **Telemetria analitica assincrona**: Metricas de performance (TTFT, latencia, bytes, status) sao extraidas e salvas em background por micro-batching sem bloquear a thread que responde ao cliente.
* **Tudo em Rust**: Distribuido como um binario unico, com baixissimo consumo de recursos (CPU/memoria) e overhead de RPS < 2% no pass-through.

---

## Como Rodar em 2 Minutos

Se voce ja tem Rust instalado:

```bash
cargo run -- --host 127.0.0.1 --port 8080
```

### Endpoints Disponiveis:
* `GET  /healthz` - Status de saude do gateway.
* `GET  /readyz` - Prontidao do gateway.
* `POST /v1/chat/completions` - Roteamento de chat streaming.

---

## Como Funciona (Arquitetura)

```text
Cliente -> oxideLLM Gateway -> Upstreams (Ollama, vLLM, Groq, etc.)
                   |
                   +--> Telemetry Ring Buffer (Memoria Lock-free)
                             |
                             +--> background worker -> telemetry_events.jsonl (Micro-batching)
```

### Principios do Projeto:
* **Desacoplamento do caminho critico**: Nenhuma gravacao em disco ou processamento pesado bloqueia a resposta do usuario.
* **Zero-copy por padrao**: Chunks SSE sao encaminhados diretamente como streams de bytes, sem deserializar o payload por token.
* **Fila de telemetria bounded**: Protecao nativa contra estouro de memoria (backpressure precoce).

---

## Guia de Configuracao

oxideLLM resolve as configuracoes com a seguinte ordem de prioridade:
1. Argumentos da linha de comando (`--port`, `--upstream-base-url`, etc.)
2. Variaveis de ambiente (`LLMK_PORT`, `LLMK_UPSTREAM_BASE_URL`, etc.)
3. Arquivo de configuracao TOML (`config.toml` na raiz)

### Exemplo de `config.toml`:
```toml
[server]
host = "127.0.0.1"
port = 8080

[[upstreams]]
id = "primary"
provider = "ollama"
base_url = "http://127.0.0.1:11434"
priority = 1

[[upstreams]]
id = "fallback"
provider = "mock"
base_url = "http://127.0.0.1:9000"
priority = 2
```

---

## Validacao Pratica

### 1. Com Provedor Real Local (Ollama)
```bash
# 1. Baixe o modelo
ollama pull tinyllama

# 2. Inicialize o gateway apontando para o Ollama
cargo run -- --upstream-provider ollama

# 3. Teste o streaming
curl -N -X POST http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "tinyllama", "messages": [{"role": "user", "content": "Ola!"}], "stream": true}'
```

### 2. Com Provedor Real Remoto (Groq)
```bash
export GROQ_API_KEY="sua-chave-aqui"
cargo run -- --upstream-base-url https://api.groq.com/openai --upstream-provider groq
k6 run -e GROQ_API_KEY=$GROQ_API_KEY k6/groq-integration.js
```

### 3. Validacao de Resiliencia (Multi-upstream & Failover)
Voce pode testar o desvio de rotas automatico do gateway apontando para um servidor primario offline e um backup online:

1. Crie um arquivo `config.toml` na raiz do projeto com a seguinte configuracao:
```toml
[server]
host = "127.0.0.1"
port = 8080

[[upstreams]]
id = "primary-dead"
provider = "mock"
base_url = "http://127.0.0.1:9000"  # Servidor inativo
priority = 1

[[upstreams]]
id = "fallback-alive"
provider = "mock"
base_url = "http://127.0.0.1:9001"  # Servidor ativo
priority = 2
```

2. Inicialize o servidor mock ativo na porta 9001:
```bash
cargo run --manifest-path mock/Cargo.toml -- --port 9001
```

3. Inicialize o gateway em outro terminal (ele lera o `config.toml` automaticamente):
```bash
cargo run
```

4. Realize a chamada ao gateway na porta 8080:
```bash
curl -N -X POST http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "mock", "messages": [{"role": "user", "content": "Teste resiliencia"}], "stream": true}'
```
O gateway falhara ao tentar se conectar com a porta 9000 (primary-dead) e, de forma transparente, desviara a chamada para o backup ativo na porta 9001 (fallback-alive), retornando o streaming normalmente.

---

## Benchmarks de Performance (WSL2 / Localhost)

O projeto contem suites de benchmark executadas sob alta carga com k6. A analise detalhada e historico estao documentados em [alpha-v1-benchmark-summary.md](file:///benchmarks/alpha-v1-benchmark-summary.md).

### Resultados Reconciliados (Stage 2):
| Caminho de execucao | RPS medio | Latencia P95 | Erros HTTP | Degradacao vs Direto |
|---|---:|---:|---:|---|
| Conexao direta ao mock Rust | 20.282,05 req/s | 59,64 ms | 0,00% | *baseline* |
| Gateway com telemetria ativa | 17.831,39 req/s | 74,40 ms | 0,00% | ~12.08% |
| Gateway (logs em `/dev/null`) | 18.014,34 req/s | 73,68 ms | 0,00% | ~11.18% |

---

## Testes Automatizados

Para garantir que o gateway esta funcionando corretamente e que nenhuma alteracao quebrou a logica existente, voce pode rodar os testes da aplicacao:

### 1. Testes Unitarios e de Integracao (Rust)
```bash
cargo test --all
```

### 2. Validador de Contexto (Documentacao)
```bash
# Executado em terminal PowerShell
.\scripts\validate_context.ps1
```

---

## Apoio & Doacoes (Sponsors)

oxideLLM e 100% gratuito e open-source. Se o gateway ajuda a reduzir custos de infraestrutura e downtime na sua organizacao, apoie o desenvolvimento voluntario:
* [GitHub Sponsors](https://github.com/sponsors/lugga1s)
* [Buy Me a Coffee](https://www.buymeacoffee.com/lugga1s)

---

## Licenca

Usa **AGPL-3.0-or-later**. Consulte [licensing-strategy.md](file:///docs/licensing-strategy.md) para detalhes da nossa politica comercial open-source.

---

<details>
<summary><b>Documentacao de Contribuicao (Clique para expandir)</b></summary>

> Os documentos abaixo sao manuais internos de engenharia, mantidos fora do repositorio publico.
> Contribuidores com acesso ao ambiente de desenvolvimento local encontram esses arquivos na raiz do projeto.

Aqui estao todos os manuais tecnicos que descrevem o funcionamento interno do gateway para desenvolvedores que desejam contribuir:

* `AGENTS.md` - Instrucoes gerais para agentes autonomos de IA.
* `docs/architecture.md` - Blueprint de arquitetura Rust.
* `docs/implementation-playbook.md` - Historico e etapas de implementacao.
* `docs/agent-task-cards.md` - Cartoes de tarefas concluidas.
* `docs/validation-gates.md` - Criterios de gates tecnicos.
* `docs/verification-ledger.md` - Registro de execucoes de teste.
* `docs/production-ritual.md` - Ritos de publicacao e semaforos.
* `docs/github-workflow.md` - Padroes de commits, PRs e tags.
* `docs/operational-priorities.md` - Filosofia de MVP funcional.
* `docs/tooling-setup.md` - Instalacao de Rust, k6 e Docker.
</details>
