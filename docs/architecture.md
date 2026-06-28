# Blueprint Tecnico Rust

Status: especificacao operacional inicial  
Publico: agentes de implementacao, engenheiro de contexto e revisores tecnicos  
Linguagem alvo: Rust

---

## 1. Visao Geral

O projeto e um gateway/proxy reverso especializado para workloads de LLM. Ele recebe chamadas HTTP, normaliza ou encaminha payloads para provedores como OpenAI-compatible, Anthropic e Ollama, devolve streaming SSE ao cliente e extrai telemetria sem bloquear a resposta.

O sistema e dividido em quatro modulos centrais:

```text
HTTP Engine
  -> Protocol Translator
  -> Telemetry Ring Buffer
  -> Batching & Persistence
```

O caminho critico e:

```text
cliente
  -> listener Rust
  -> roteamento
  -> upstream
  -> streaming SSE
  -> cliente
```

O caminho analitico e:

```text
request context
  -> evento compacto
  -> ring/fila bounded
  -> worker background
  -> lote Parquet/DuckDB/ClickHouse
```

Regra: o caminho analitico nunca deve bloquear o caminho critico em modo padrao.

---

## 2. Decisao de Linguagem: Rust

Rust foi escolhido porque o projeto precisa maximizar previsibilidade de latencia e controle de memoria em streaming de alta concorrencia.

Motivos:

- ausencia de garbage collector;
- ownership e lifetimes ajudam a impedir uso incorreto de buffers;
- `Bytes` permite compartilhamento barato de chunks;
- compilador impede data races em codigo seguro;
- ecossistema async maduro com Tokio;
- bom encaixe para parsers zero-copy e estruturas bounded.

Go continua tecnicamente viavel para um gateway rapido, mas Rust se alinha melhor ao posicionamento de baixa latencia, controle de heap e engenharia de performance.

Claim permitido:

```text
Rust reduz uma classe de jitter associada a GC e permite controle mais direto de alocacoes.
```

Claim proibido sem benchmark:

```text
Rust sera sempre mais rapido que Go em todos os cenarios.
```

---

## 3. Stack Inicial

| Area | Escolha inicial | Razao |
|---|---|---|
| Runtime async | Tokio | runtime async padrao de mercado em Rust |
| HTTP MVP | Axum sobre Hyper | produtividade, compatibilidade Tower, base Hyper |
| HTTP core futuro | Hyper ou Pingora | reavaliar quando proxy L7 amadurecer |
| Buffers | bytes::Bytes / BytesMut | passagem barata de chunks |
| Telemetria interna | tracing + metricas | spans/logs estruturados, sampling |
| Fila hot path | crossbeam ArrayQueue ou equivalente | bounded, baixa contencao |
| Bench HTTP | k6 | VUs, thresholds, relatorios |
| Heap profiling | heaptrack / DHAT | alocacoes por request |
| Persistencia local | Parquet ou DuckDB | local-first, colunar |
| Persistencia server | ClickHouse | analitica de alta cardinalidade |

---

## 4. Modulo 1: HTTP Engine

Responsabilidades:

- abrir socket;
- aceitar conexoes;
- expor endpoints OpenAI-compatible;
- preservar streaming;
- aplicar timeout e cancelamento;
- propagar disconnect do cliente;
- encaminhar headers allowlisted;
- medir bytes in/out.

Endpoints MVP:

```text
GET  /healthz
GET  /readyz
POST /v1/chat/completions
```

Endpoints futuros:

```text
POST /v1/completions
POST /v1/embeddings
POST /v1/responses
GET  /metrics
GET  /admin/routes
```

O engine HTTP nao deve:

- escrever em banco;
- parsear prompt completo para logging;
- esperar flush de telemetria;
- criar uma task por token;
- transformar todo chunk SSE em objeto JSON completo.

---

## 5. Modulo 2: Protocol Translator

Responsabilidades:

- detectar formato de entrada;
- extrair campos minimos;
- fazer pass-through quando possivel;
- transcodificar apenas quando necessario;
- processar SSE como stream de bytes;
- extrair usage e TTFT sem reter corpo completo.

Formatos iniciais:

```text
OpenAI-compatible -> OpenAI-compatible: pass-through
OpenAI-compatible -> Ollama OpenAI-compatible: pass-through/adaptacao leve
OpenAI-compatible -> Anthropic: fase posterior
Anthropic -> OpenAI-compatible: fase posterior
```

Representacao canonica minima:

```text
CanonicalRequestView
  request_id
  tenant_id
  route_id
  provider_id
  model
  stream
  max_tokens
  body_ref
```

A view nao deve virar um objeto generico gigante. O corpo original deve permanecer como bytes sempre que possivel.

---

## 6. Modulo 3: Telemetry Ring Buffer

Responsabilidades:

- receber eventos compactos;
- operar com capacidade fixa;
- publicar sem bloquear o cliente;
- expor contador de drops;
- separar eventos finais de eventos de debug.

Modelo:

```text
MPSC producers: request tasks
SPSC/consumer: telemetry worker
capacity: configuravel
on_full: drop debug, preservar contador, politica especial para billing
```

Evento final:

```text
TelemetryFinalEvent
  request_id
  tenant_id
  route_id
  provider_id
  model_id
  started_at
  completed_at
  ttft_ms
  total_latency_ms
  input_tokens
  output_tokens
  bytes_in
  bytes_out
  status
  error_class
```

Nao armazenar:

- prompt completo por padrao;
- completion completa por padrao;
- API keys;
- headers sensiveis;
- referencias a buffers devolvidos ao pool.

---

## 7. Modulo 4: Batching & Persistence

Responsabilidades:

- consumir eventos em background;
- montar lotes;
- persistir em sink local/server;
- expor lag, batch size, flush duration e drops;
- fazer retry sem chamar de volta o request.

Gates de batching:

```text
flush por tempo: 500ms inicial
flush por tamanho: 1000 eventos inicial
flush por bytes: definir apos benchmark
```

Sinks:

```text
Fase 1: memoria/arquivo JSONL apenas para debug
Fase 2: Parquet local
Fase 3: DuckDB local
Fase 4: ClickHouse opcional
```

---

## 8. Fluxo de Request

```text
T0 recebe headers
T1 cria request_id e accumulator
T2 parse minimo do body
T3 resolve rota/provedor/modelo
T4 abre upstream
T5 recebe headers upstream
T6 primeiro delta util define TTFT
T7 encaminha chunks ao cliente
T8 captura usage final quando existir
T9 publica evento final no ring buffer
T10 libera recursos
```

O cliente espera somente o necessario para receber resposta. Ele nao espera persistencia analitica.

---

## 9. Backpressure

Backpressure obrigatorio:

- limite global de inflight requests;
- limite por provedor;
- limite por tenant;
- limite de payload;
- limite de streams por conexao upstream;
- limite de fila de telemetria;
- timeout de admissao.

Quando exceder:

```text
429 Too Many Requests
Retry-After quando aplicavel
evento de telemetria de rejeicao se fila permitir
```

---

## 10. Reavaliacao Pingora

Pingora, da Cloudflare, e uma opcao relevante para proxy L7 em Rust. Ela deve ser reavaliada quando:

- o MVP Axum/Hyper demonstrar overhead acima do gate;
- precisarmos de recursos avancados de proxy;
- a camada de roteamento L7 crescer;
- connection pooling e HTTP/2/HTTP/3 exigirem controle mais profundo.

Antes disso, Axum/Hyper e mais simples para construir o vertical slice rapidamente.

---

## 11. Antipadroes Proibidos

- Salvar log no Postgres antes de responder cliente.
- Usar `serde_json::Value` para cada chunk SSE.
- Converter `Bytes` grandes para `String` sem necessidade.
- Criar task por token.
- Ter fila de telemetria sem limite.
- Ignorar client disconnect.
- Fazer benchmark somente contra o gateway sem baseline direto.
- Publicar comparacao de performance sem comando reproduzivel.

---

## 12. Definicao de Pronto do MVP

O MVP tecnico esta pronto quando:

- `cargo check`, `cargo test`, `cargo clippy` passam;
- gateway responde `/healthz` e `/readyz`;
- `/v1/chat/completions` suporta SSE mock ou upstream;
- k6 compara direto vs gateway;
- RPS degrada menos de 2% no pass-through mock;
- P99 fica flat contra baseline direto;
- heap profiling nao mostra alocacao por chunk em pass-through;
- telemetria e publicada em buffer bounded;
- buffer cheio nao bloqueia resposta;
- relatorio de benchmark existe em `benchmarks/results/`.

## 13. Definicao de "Flat com vLLM Nativo"

O projeto pode dizer que manteve performance flat com vLLM nativo somente quando:

```text
vLLM direto e gateway -> vLLM foram testados no mesmo hardware;
mesmo script k6;
mesmo modelo;
mesma concorrencia;
mesma janela de tempo;
RPS do gateway >= 98% do direto;
P99 nao apresenta regressao material;
TTFT tem overhead pequeno e documentado;
artefatos estao salvos.
```

Essa e uma meta agressiva. Se o resultado ficar entre 95% e 98%, o status e amarelo: tecnicamente promissor, mas ainda nao pronto para claim publico de paridade.
