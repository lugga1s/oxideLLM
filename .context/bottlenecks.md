# Especificacao Mestre de Gargalos e Design da Camada Proxy Base

Codinome: **oxideLLM**  
Status: **documento tecnico mestre / base de arquitetura**  
Escopo: **gateway de IA de alta performance para roteamento, streaming, normalizacao de protocolo e telemetria sem bloqueio**

---

## 1. Sumario Executivo

Este documento consolida a engenharia reversa dos gargalos observados em gateways tradicionais de IA e define a arquitetura base da nossa camada proxy de alta performance. O objetivo e eliminar a degradacao severa causada por interpretadores tradicionais, logging/tracing sincrono e escrita direta em bancos relacionais no caminho critico da requisicao.

Nos testes internos com 500 requisicoes concorrentes, a degradacao observada foi:

| Caminho de execucao | Vazao aproximada | Eficiencia relativa | Degradacao |
|---|---:|---:|---:|
| Conexao direta ao motor de inferencia vLLM | ~16,0 req/s | 100,0% | 0,0% |
| Gateway tradicional com 4 workers, Postgres e Redis | ~8,8 req/s | 55,0% | -45,0% |
| Gateway tradicional com 1 worker, Postgres e Redis | ~3,9 req/s | 24,4% | -75,6% |

A conclusao arquitetural e direta: o gargalo principal nao esta apenas no modelo de inferencia, mas na camada intermediaria que mistura proxy, transformacao de payload, tracing, persistencia, contabilidade e controle de concorrencia dentro do mesmo caminho sincrono.

Nossa fundacao deve separar rigidamente:

1. **Plano de dados**: aceita a requisicao, normaliza protocolo, encaminha ao provedor, transmite stream e encerra a conexao do cliente com a menor quantidade possivel de copias e alocacoes.
2. **Plano de controle**: resolve politicas de roteamento, cotas, chaves, retries, fallback e circuit breakers.
3. **Plano de telemetria**: recebe eventos compactos e imutaveis a partir do plano de dados, agrega em background e persiste por lote fora do caminho critico.

A decisao central deste design e que a thread, task ou future responsavel pelo cliente nunca deve aguardar escrita em banco, flush de log, serializacao pesada de tracing ou confirmacao de pipeline analitico. O proxy deve liberar o cliente assim que o ultimo byte util for transmitido ou assim que o erro terminal for propagado.

---

## 2. Diagnostico do Gargalo Corporativo

### 2.1 Sintoma

Gateways baseados em Python/Gunicorn, ou arquiteturas equivalentes centradas em interpretadores e workers de processo, apresentam queda brusca de vazao sob alta concorrencia quando combinam:

- parse completo de payloads JSON no caminho critico;
- conversao de objetos dinamicos para modelos internos;
- tracing sincrono por request;
- logging estruturado com serializacao por evento;
- persistencia direta em PostgreSQL;
- uso de Redis como coordenador de estado quente;
- workers limitados que fazem proxy e telemetria ao mesmo tempo;
- streaming SSE processado como strings, linhas ou objetos de alto nivel.

O resultado e uma fila interna formada por trabalho que nao pertence ao caminho minimo de entrega do token ao cliente.

### 2.2 Causa raiz

O colapso de performance e produzido por quatro mecanismos acoplados:

1. **Saturacao de E/S em banco relacional**
   - Cada request gera multiplas escritas ou updates.
   - Writes pequenos e frequentes competem por WAL, locks, indices e fsync.
   - A latencia de banco passa a contaminar a latencia percebida pelo cliente.

2. **Pressao de alocacao e coleta de lixo**
   - Payloads sao materializados em objetos intermediarios.
   - Eventos de stream sao concatenados ou convertidos para strings.
   - Metadados sao copiados entre camadas de framework.
   - Em Go, isso aumenta a pressao no garbage collector quando buffers escapam para heap.
   - Em Rust, isso aparece como clonagem excessiva de `String`, `Vec<u8>` e `serde_json::Value`.

3. **Modelo de concorrencia com workers bloqueantes**
   - Um worker que deveria apenas retransmitir bytes tambem espera banco, Redis, logger ou callback.
   - Quando os workers saturam, a fila de conexoes aumenta e o P99 degrada antes da media.

4. **Ausencia de backpressure formal**
   - Sem limites por rota, provedor, tenant e conexao, o sistema aceita mais trabalho do que consegue completar.
   - A degradacao deixa de ser controlada e passa a ser uma queda em cascata.

### 2.3 Implicacao de arquitetura

O gateway nao pode ser tratado como uma aplicacao web tradicional. Ele deve ser tratado como um **proxy de protocolo em tempo real**, com caracteristicas proximas a balanceadores L7, proxies reversos e runtimes de streaming.

O desenho correto favorece:

- buffers reaproveitados;
- leitura e escrita incremental;
- parse seletivo;
- estruturas com ownership claro;
- conexoes persistentes;
- multiplexacao HTTP/2 e HTTP/3;
- filas lock-free ou com baixa contencao;
- telemetria assincrona por eventos compactos;
- persistencia por lote, nao por request.

---

## 3. Objetivos e Nao Objetivos

### 3.1 Objetivos

- Preservar a maior parte possivel da eficiencia da conexao direta ao motor de inferencia.
- Reduzir overhead do gateway para uma faixa previsivel e mensuravel.
- Estabilizar P95/P99 sob picos de concorrencia.
- Suportar OpenAI, Anthropic e Ollama por meio de uma representacao canonica interna.
- Encaminhar SSE com latencia minima por chunk.
- Extrair metadados sem bloquear a resposta do cliente.
- Evitar escrita sincrona em PostgreSQL no caminho critico.
- Permitir implementacao em Go ou Rust sem mudar os invariantes arquiteturais.
- Manter uma superficie compativel com alta disponibilidade: timeouts, retries, circuit breakers, health checks e drenagem graciosa.

### 3.2 Nao objetivos

- Nao executar inferencia local dentro do gateway.
- Nao transformar o gateway em data warehouse operacional.
- Nao depender de Postgres para cada evento de token.
- Nao normalizar todos os provedores por um objeto JSON generico pesado.
- Nao garantir telemetria analitica exatamente uma vez no caminho quente.
- Nao fazer contagem perfeita de tokens quando o provedor nao fornece dados ou quando a tokenizer local nao esta disponivel; nesses casos a contagem deve ser marcada como estimada.

---

## 4. Principios Arquiteturais

### 4.1 Caminho critico minimo

O caminho critico da requisicao deve conter apenas:

1. Aceitar conexao.
2. Validar autenticacao e politica minima.
3. Escolher rota/provedor/modelo.
4. Traduzir payload de entrada quando necessario.
5. Encaminhar request ao upstream.
6. Ler resposta upstream.
7. Repassar resposta ao cliente.
8. Emitir eventos de telemetria para fila assincrona sem aguardar persistencia.

Qualquer trabalho fora desses passos deve ser deslocado para background.

### 4.2 Zero-copy como direcao, nao dogma

"Zero-copy" aqui significa minimizar copias no caminho quente, nao prometer ausencia absoluta de copia. Em HTTP, TLS, JSON e SSE sempre havera pontos onde alguma copia pode ser necessaria. O principio e:

- nao materializar corpos inteiros quando streaming basta;
- nao converter bytes para string sem necessidade;
- nao criar objetos intermediarios genericos;
- nao copiar campos grandes quando slices/borrowed references bastam;
- nao reserializar payload compativel quando o provedor ja aceita o formato de entrada;
- alocar somente quando a fronteira semantica exige transformacao.

### 4.3 Telemetria por evento, persistencia por lote

O request deve emitir eventos pequenos para um buffer assincrono. Um worker separado faz batch, compressao, enriquecimento e flush para o destino analitico.

Regra: **a persistencia de telemetria nunca esta no caminho de resposta do cliente**.

### 4.4 Backpressure explicito

O gateway deve recusar, atrasar ou degradar comportamento antes de entrar em colapso. Backpressure deve existir em:

- limite global de conexoes;
- limite por tenant;
- limite por provedor;
- limite por modelo;
- limite por rota;
- limite de streams HTTP/2/HTTP/3;
- tamanho maximo de payload;
- tamanho maximo de fila de telemetria;
- limite de memoria por processo.

### 4.5 Estado quente em memoria, estado frio fora do caminho

Decisoes de roteamento, cotas de curta janela e circuit breakers devem usar estruturas em memoria com atualizacao barata. Persistencia definitiva, auditoria e analitica ficam fora do caminho quente.

---

## 5. Design Espacial da Fundacao

### 5.1 Planos do sistema

```text
                  +----------------------+
                  |      Control Plane   |
                  | routing, policy,     |
                  | quotas, breakers     |
                  +----------+-----------+
                             |
                             v
+---------+       +----------+-----------+       +-------------+
| Client  | <---> |       Data Plane     | <---> | Providers   |
| SDK/API |       | protocol proxy, SSE, |       | OpenAI,     |
|         |       | buffers, pooling     |       | Anthropic,  |
+---------+       +----------+-----------+       | Ollama      |
                             |                   +-------------+
                             v
                  +----------+-----------+
                  |    Telemetry Plane   |
                  | async events, batch, |
                  | sinks, sampling      |
                  +----------------------+
```

### 5.2 Fronteiras de responsabilidade

| Camada | Responsabilidade | Nao deve fazer |
|---|---|---|
| Listener | aceitar conexoes, TLS, protocolo HTTP | persistir metrica |
| Router | resolver tenant, chave, modelo, provedor | parsear stream inteiro |
| Translator | adaptar formato canonico/provedor | escrever em banco |
| Stream Pump | mover bytes entre upstream e cliente | bloquear em logger |
| Metering Tap | extrair metadados incrementais | alterar fluxo primario |
| Telemetry Queue | receber evento compacto | aplicar regra de negocio pesada |
| Telemetry Worker | agregar, comprimir, persistir | reter task do cliente |

### 5.3 Invariante de isolamento

A task que possui o socket do cliente pode publicar telemetria por operacao nao bloqueante. Se a fila estiver cheia, a decisao deve ser uma destas, configuravel por criticidade:

1. descartar metrica de baixa prioridade e incrementar contador de perda;
2. fazer sampling adaptativo;
3. escrever evento minimo em WAL local assincrono;
4. aplicar backpressure somente quando a politica corporativa exigir auditoria obrigatoria.

O modo padrao para alta performance e **nao bloquear cliente por telemetria analitica**.

---

## 6. Pilar 1: Arquitetura de Proxy Zero-Copy

### 6.1 Objetivo

Construir uma camada de transicao de protocolo capaz de receber payloads nos formatos OpenAI, Anthropic e Ollama, converter para uma representacao canonica minima e encaminhar ao provedor selecionado com o menor numero possivel de alocacoes, copias e serializacoes.

### 6.2 Modelo canonico interno

A representacao canonica nao deve ser um `map[string]interface{}` nem um `serde_json::Value` completo por padrao. Ela deve ser uma estrutura enxuta com campos referenciados por fatias de bytes ou strings emprestadas quando possivel.

Exemplo conceitual:

```text
CanonicalRequest
  request_id
  tenant_id
  route_id
  provider_hint
  model
  messages_ref
  prompt_ref
  tools_ref
  stream
  temperature
  max_tokens
  raw_body_ref
  body_encoding
```

Campos grandes devem permanecer como referencias ao corpo original enquanto a vida util do buffer permitir. Campos pequenos e necessarios para roteamento podem ser copiados para uma estrutura compacta.

### 6.3 Regras de transformacao por provedor

| Entrada | Saida escolhida | Estrategia |
|---|---|---|
| OpenAI -> provedor OpenAI-compatible | pass-through quando possivel | nao reserializar corpo; apenas ajustar headers e URL |
| OpenAI -> Anthropic | transcodificacao seletiva | parse parcial de `model`, `messages`, `stream`, `tools`; writer incremental |
| Anthropic -> OpenAI-compatible | transcodificacao seletiva | mapear `messages`, `system`, `max_tokens`, `stream` |
| Ollama -> OpenAI-compatible | adaptador leve | converter campos de prompt/chat e stream |
| Ollama -> Ollama | pass-through | preservar corpo quando politicas permitirem |

Regra pratica: se o formato de entrada ja e aceito pelo upstream, o proxy deve encaminhar o corpo original sem rebuild de JSON.

### 6.4 Parse seletivo de JSON

O parse completo do payload deve ser evitado. A camada de roteamento normalmente precisa apenas de:

- `model`;
- `stream`;
- tenant/chave;
- tamanho aproximado do corpo;
- campos de politica como `max_tokens`;
- provider override quando permitido;
- atributos de tracing de entrada.

Para esses campos, usar parse seletivo:

- Em Go:
  - scanner baseado em `[]byte`;
  - bibliotecas de JSON de baixa alocacao quando justificadas;
  - evitar `map[string]any` no caminho quente;
  - evitar conversoes `string(body)` para busca de campo;
  - manter slices de `[]byte` para campos ate a etapa de escrita.

- Em Rust:
  - structs com lifetime e `Cow<'a, str>`;
  - `serde` com borrowing quando o schema for conhecido;
  - parser parcial quando apenas poucos campos forem necessarios;
  - evitar `serde_json::Value` para o corpo completo;
  - usar `bytes::Bytes`/`BytesMut` para ownership barato e referencia contada.

### 6.5 Modelo de ownership e heap

#### Go

Em Go, o risco e fazer buffers escaparem para heap e aumentar a pressao no GC. Diretrizes:

- manter buffers em pools por tamanho;
- evitar capturar buffers grandes em closures;
- nao armazenar `[]byte` de request em estruturas globais;
- copiar apenas campos pequenos que sobrevivem ao request;
- retornar buffers ao pool em `defer` controlado no fim da task;
- usar structs concretas em vez de interfaces no caminho quente quando possivel;
- medir escape analysis com `go build -gcflags=-m`.

Padrao recomendado:

```text
RequestTask
  acquire buffer
  read/route/forward
  emit telemetry snapshot compacta
  release buffer
```

A telemetria nao deve carregar slices do body original depois que o buffer for devolvido ao pool. Ela deve receber somente valores pequenos e owned: ids, contadores, timestamps, enum de rota e codigos de status.

#### Rust

Em Rust, o risco e degradar para clones defensivos. Diretrizes:

- representar payloads com `Bytes` quando o dado precisa sobreviver entre tasks;
- usar borrowed structs durante parse local;
- promover para owned somente na fronteira assincrona;
- nao clonar `String` de prompt/mensagens para telemetria;
- separar lifetimes de parsing e lifetimes de background;
- usar `Arc<str>` apenas para dados compartilhados e de baixa cardinalidade;
- preferir enums compactos para provider/model route quando catalogados.

Padrao recomendado:

```text
Bytes body
  -> borrowed parse view
  -> route decision
  -> streaming writer
  -> compact owned telemetry event
```

### 6.6 Transcodificacao sem objeto intermediario pesado

Quando a traducao for inevitavel, o fluxo deve ser:

```text
input bytes
  -> parser seletivo/borrowed view
  -> provider-specific streaming encoder
  -> upstream request body
```

Nao usar:

```text
input bytes
  -> generic JSON object
  -> canonical JSON object
  -> provider JSON object
  -> output bytes
```

Esse segundo fluxo multiplica alocacoes, copias, misses de cache e trabalho do GC.

### 6.7 SSE como maquina de estados

SSE deve ser processado como stream de bytes, nao como lista de strings.

O parser deve reconhecer:

- delimitador de evento: linha vazia (`\n\n` ou `\r\n\r\n`);
- campo `data:`;
- campo `event:`;
- comentarios iniciados por `:`;
- evento terminal `[DONE]`;
- fragmentos que cruzam fronteira de buffer.

Modelo:

```text
Read upstream chunk
  scan bytes for line/event boundaries
  forward bytes to client immediately
  feed metering tap with slices do chunk
  retain only fragmento incompleto
```

### 6.8 Leitura direta de buffers de rede

#### Go

Usar modelo conceitual com `io.Reader`:

```text
buf := pool.Acquire()
for {
    n, err := upstream.Body.Read(buf)
    if n > 0 {
        chunk := buf[:n]
        sse.Scan(chunk)
        client.Write(chunk)
        flusher.Flush()
    }
    if err == EOF { break }
    if err != nil { handle }
}
pool.Release(buf)
```

Pontos importantes:

- `chunk` nao deve ser armazenado apos a iteracao.
- O metering tap deve copiar somente contadores e pequenos campos.
- Fragments incompletos devem usar scratch buffer pequeno e limitado.
- Flush deve ser calibrado para streaming: rapido para tokens, mas sem syscall excessiva quando o provedor envia fragmentos minusculos.

#### Rust

Usar modelo conceitual com `AsyncRead`/`Stream`:

```text
while let Some(chunk) = upstream.next().await {
    let bytes = chunk?;
    sse.scan(&bytes);
    client.send(bytes.clone()).await?;
}
```

Quando `Bytes` e usado, `clone()` e incremento de referencia, nao copia do conteudo. Ainda assim, deve-se evitar reter muitos chunks em filas.

### 6.9 Encaminhamento de bytes e preservacao de latencia

O proxy deve priorizar o encaminhamento imediato do chunk ao cliente. O processamento de metadados deve ser um tap lateral:

```text
upstream bytes
    |
    +--> client writer
    |
    +--> metering tap
```

O tap nao pode ser dono do fluxo. Se o tap falhar, o stream primario continua.

### 6.10 Compressao

Para streaming de tokens, compressao deve ser tratada com cautela:

- compressao pode aumentar latencia de primeiro token;
- buffers de compressao podem reter bytes aguardando janela;
- SSE geralmente deve favorecer flush e baixa latencia;
- se habilitada, deve ser por politica de cliente/rota e medida no P99.

### 6.11 Headers e protocolo

O proxy deve normalizar:

- `Authorization`;
- `Content-Type`;
- `Accept`;
- `Accept-Encoding`;
- `User-Agent`;
- headers de tracing;
- headers de tenant;
- timeout/deadline;
- idempotency key quando aplicavel.

Headers de cliente nao confiaveis nao devem vazar diretamente ao provedor. O conjunto encaminhado deve ser allowlisted.

### 6.12 Erros de stream

Erros devem preservar a semantica do protocolo:

- erro antes dos headers: retornar status HTTP adequado;
- erro apos inicio do stream: emitir evento SSE de erro quando o protocolo permitir;
- upstream reset: encerrar stream e registrar causa;
- client disconnect: cancelar upstream imediatamente;
- timeout: cancelar upstream e emitir telemetria de timeout.

### 6.13 Criterios de aceitacao do pilar zero-copy

- Nenhum parse completo de JSON no caminho pass-through.
- Nenhuma conversao body inteiro para string.
- Nenhum objeto generico por token SSE.
- Buffers devolvidos ao pool apos request.
- TTFT medido sem reter stream.
- P99 nao cresce linearmente com concorrencia por alocacao.
- Profiles de CPU/memoria demonstram queda de alocacoes por request em relacao ao gateway tradicional.

---

## 7. Pilar 2: Arquitetura de Pooling e Concorrencia Reduzida

### 7.1 Objetivo

Reduzir a quantidade de trabalho concorrente real dentro do processo, mantendo alta concorrencia externa por multiplexacao de streams, conexoes persistentes, backpressure e reaproveitamento de memoria.

Alta concorrencia de clientes nao deve significar numero equivalente de conexoes upstream, goroutines/tasks ativas fazendo trabalho pesado, buffers alocados ou writes simultaneos em banco.

### 7.2 Modelo de rede

O gateway deve suportar:

- HTTP/1.1 para compatibilidade;
- HTTP/2 para multiplexacao de streams sobre conexoes persistentes;
- HTTP/3/QUIC quando o ambiente exigir reducao de head-of-line blocking em redes instaveis.

Preferencia operacional:

1. Cliente -> Gateway: HTTP/2 ou HTTP/3 quando disponivel.
2. Gateway -> Provedor: HTTP/2 quando o provedor suporta multiplexacao.
3. Gateway -> Provedor local, como vLLM/Ollama: conexoes persistentes, keep-alive e limite de sockets por instancia.

### 7.3 Multiplexacao de conexoes

Com HTTP/2 e HTTP/3, multiplos streams logicos compartilham uma conexao fisica. Isso reduz:

- custo de handshake TCP/TLS;
- sockets abertos;
- buffers por conexao;
- contencao no scheduler;
- overhead de kernel;
- jitter de latencia por criacao de conexao.

O pool upstream deve ser indexado por:

```text
provider_id
region
model_family
protocol
auth_profile
```

Nao misturar tenants ou credenciais em conexoes quando a politica de isolamento exigir segregacao.

### 7.4 Limites de streams

Cada conexao HTTP/2/HTTP/3 deve respeitar:

- limite maximo negociado pelo peer;
- limite interno conservador por provedor;
- limite por classe de modelo;
- limite por tenant;
- limite por prioridade.

Exemplo conceitual:

```text
provider.openai.max_connections = 64
provider.openai.max_streams_per_connection = 100
provider.openai.max_inflight_requests = 5000
tenant.enterprise_a.max_inflight_requests = 800
model.gpt-large.max_inflight_requests = 300
```

Quando o limite for atingido, usar fila curta e timeout de admissao. Fila sem limite e apenas colapso adiado.

### 7.5 Concorrencia reduzida por work classes

Nem todo trabalho deve compartilhar o mesmo pool de execucao.

Classes recomendadas:

| Classe | Exemplos | Prioridade | Bloqueia cliente? |
|---|---|---:|---|
| Data path | proxy, stream, cancelamento | maxima | sim |
| Control path | roteamento, cota quente, breaker | alta | sim, mas curto |
| Telemetry hot | enfileirar evento compacto | alta | nao deve bloquear |
| Telemetry cold | batch, flush, compressao | baixa/media | nao |
| Maintenance | health check, refresh de config | baixa | nao |

Essa separacao evita que flush analitico consuma o scheduler necessario para transmitir tokens.

### 7.6 Modelo de backpressure

Backpressure deve acontecer em camadas:

1. **Admissao**
   - rejeitar cedo se tenant/modelo/provedor excedeu limite;
   - retornar `429` ou erro equivalente com `Retry-After`;
   - evitar aceitar request que ja esta fadado a timeout.

2. **Upstream**
   - limitar streams por conexao;
   - abrir nova conexao apenas se pool permitir;
   - aplicar circuit breaker quando erro/timeout sobe.

3. **Memoria**
   - limitar buffers vivos;
   - limitar payload maximo;
   - usar pools por classe de tamanho;
   - recusar requests que excedem envelope.

4. **Telemetria**
   - fila bounded;
   - sampling adaptativo;
   - drop controlado de eventos nao criticos;
   - contador de perda sempre preservado.

### 7.7 Pooling de memoria

Buffers devem ser organizados por tamanho para evitar reter grandes alocacoes desnecessariamente:

```text
4 KiB   -> headers, pequenos chunks SSE
16 KiB  -> leitura padrao de stream
64 KiB  -> payloads medios
256 KiB -> payloads grandes controlados
>256 KiB -> alocacao dedicada, sem retorno ao pool padrao
```

Regras:

- buffer grande nao volta para pool pequeno;
- buffer acima do limite e descartado apos uso;
- pools devem ser observaveis por metricas;
- pool nao e cache infinito;
- buffers devem ser zerados apenas quando contem dados sensiveis ou quando politica exigir.

### 7.8 Request/Response pooling

Objetos de request/response devem ser reaproveitados, mas com cuidado para nao vazar estado entre tenants.

Estrutura conceitual:

```text
PooledRequestContext
  request_id
  tenant_id
  route_id
  provider_id
  deadline
  cancellation
  counters
  scratch
  canonical_view
```

Ao devolver ao pool:

- limpar ids;
- limpar referencias ao body;
- limpar headers sensiveis;
- zerar contadores;
- limpar estado de parser SSE;
- preservar apenas buffers internos seguros para reuso.

### 7.9 Evitar falso compartilhamento e contencao

Contadores de alta frequencia devem ser:

- agregados localmente por worker/shard;
- publicados periodicamente;
- alinhados para evitar false sharing quando necessario;
- atualizados por atomics apenas quando o custo for aceitavel.

Nao usar mutex global para:

- contagem de tokens;
- total de requests;
- bytes transmitidos;
- eventos de stream;
- selecao de upstream em hot path.

### 7.10 Sharding interno

Para reduzir contencao, estruturas quentes podem ser particionadas por:

- tenant hash;
- provider id;
- route id;
- CPU/core worker;
- connection group.

Exemplo:

```text
telemetry_queue[shard_id]
rate_limiter[tenant_hash % N]
connection_pool[provider_id][region]
breaker[provider_id][model_family]
```

### 7.11 Scheduler e runtime

#### Go

Go oferece goroutines baratas, mas isso nao significa que todas as etapas devem criar goroutines. Diretrizes:

- uma goroutine por request/stream e aceitavel ate o limite de memoria e scheduler;
- evitar goroutine por token;
- evitar goroutine por evento de telemetria;
- usar channels bounded;
- medir bloqueios com `go tool trace` e mutex/block profiles;
- controlar `GOMAXPROCS` conforme CPU e workload;
- evitar chamadas bloqueantes de banco no mesmo pool de execucao do proxy.

#### Rust

Rust com Tokio ou runtime equivalente exige cuidado com tarefas que bloqueiam:

- nao bloquear executor async com escrita sincrona;
- usar `spawn_blocking` apenas para trabalho CPU/bloqueante isolado;
- evitar task por token;
- usar `mpsc` bounded para telemetria;
- medir tempo de poll e saturacao do reactor;
- usar `Bytes` para passagem eficiente de chunks;
- aplicar cancellation por `select!` entre client disconnect, upstream e deadline.

### 7.12 Estabilidade de P99

O P99 sera estabilizado quando:

- o numero de alocacoes por request for previsivel;
- o numero de conexoes upstream for controlado;
- filas tiverem limite;
- telemetria nao bloquear;
- retries forem orcamentados por deadline;
- circuit breakers evitarem provedores degradados;
- o proxy cancelar upstream quando o cliente desconecta;
- writes pequenos forem coalescidos quando possivel sem prejudicar TTFT.

### 7.13 Retries e hedging

Retries podem amplificar carga se forem ingenuos. Politica:

- retry somente antes de iniciar stream ao cliente;
- apos primeiro byte enviado, retry automatico normalmente e proibido;
- retry deve respeitar deadline do cliente;
- usar jitter;
- circuit breaker por erro e latencia;
- hedging apenas para rotas idempotentes e com orcamento explicito.

### 7.14 Timeouts

Separar:

- timeout de admissao;
- timeout de conexao upstream;
- timeout de envio de headers;
- timeout ate primeiro token;
- timeout entre chunks;
- deadline total;
- timeout de drenagem no shutdown.

Essa separacao permite diagnostico preciso e evita matar streams longos saudaveis.

### 7.15 Criterios de aceitacao do pilar de concorrencia

- Pool upstream reutiliza conexoes sob carga.
- HTTP/2 multiplexa multiplos streams por conexao quando suportado.
- Filas internas sao bounded.
- Nao ha goroutine/task por token.
- Backpressure retorna erro cedo em vez de colapsar.
- P99 permanece dentro de envelope definido mesmo quando concorrencia aumenta.
- Memory profiles nao mostram crescimento sem limite de buffers.

---

## 8. Pilar 3: Telemetria sem Bloqueio

### 8.1 Objetivo

Extrair metadados de uso, latencia, rota e resultado sem bloquear a transmissao do stream nem aguardar escrita em banco. A telemetria deve ser precisa o suficiente para billing, auditoria operacional e analise de performance, mas a coleta nao pode recriar o gargalo que queremos eliminar.

### 8.2 Eventos principais

O plano de dados deve emitir eventos compactos:

```text
RequestStarted
UpstreamSelected
UpstreamHeadersReceived
FirstTokenObserved
ChunkObserved
UsageObserved
RequestCompleted
RequestFailed
ClientDisconnected
```

Nem todos precisam ser persistidos individualmente. Muitos podem ser agregados em um unico registro final.

### 8.3 Metadados obrigatorios

| Campo | Origem | Momento de extracao |
|---|---|---|
| `request_id` | gerado no ingresso | antes de roteamento |
| `tenant_id` | chave/API token | admissao |
| `route_id` | roteador | apos decisao de rota |
| `provider_id` | roteador | antes de abrir upstream |
| `model_requested` | payload/header | parse seletivo inicial |
| `model_resolved` | roteador | apos politica/fallback |
| `input_tokens` | tokenizer local ou usage upstream | antes do upstream se local; senao ao final |
| `output_tokens` | usage upstream ou contador incremental | durante stream/final |
| `ttft_ms` | relogio monotonico | no primeiro delta util encaminhado |
| `total_latency_ms` | relogio monotonico | fim do stream ou erro |
| `status_code` | HTTP/protocolo | headers/fim |
| `error_class` | mapper interno | no erro |
| `bytes_in` | contador de body | leitura de request |
| `bytes_out` | contador de stream | escrita ao cliente |
| `started_at` | wall clock | ingresso |
| `completed_at` | wall clock | fim |
| `deadline_ms` | contexto | admissao |
| `retry_count` | politica upstream | antes/fim |
| `cache_status` | control plane | quando aplicavel |

### 8.4 Momento exato de extracao

#### T0: ingresso da requisicao

Ao receber headers e iniciar body:

- gerar `request_id`;
- capturar `started_at` em wall clock;
- capturar `start_mono` em relogio monotonico;
- identificar tenant/chave;
- inicializar contadores de bytes;
- criar `TelemetryAccumulator` local no request context.

Nao escrever em banco.

#### T1: parse minimo de entrada

Durante leitura ou buffering controlado do corpo:

- extrair `model_requested`;
- extrair `stream`;
- extrair `max_tokens` se necessario para politica;
- calcular `bytes_in`;
- se tokenizer local estiver habilitada e barata para a rota, calcular `input_tokens`;
- se tokenizer for cara, adiar ou marcar como estimada.

Nao transformar corpo inteiro em objeto generico.

#### T2: decisao de rota

Apos o control plane resolver rota:

- gravar `route_id`;
- gravar `provider_id`;
- gravar `model_resolved`;
- gravar politica aplicada;
- gravar se houve fallback ou override.

Emitir evento assincrono `UpstreamSelected` somente se necessario para observabilidade em tempo real. Caso contrario, manter no acumulador local.

#### T3: envio ao upstream

Antes de abrir/enviar upstream:

- iniciar contador de tentativa;
- registrar `upstream_start_mono`;
- anexar deadline/cancellation.

Nao publicar evento pesado.

#### T4: headers do upstream

Quando headers chegam:

- capturar status upstream;
- capturar headers de request id do provedor se existirem;
- classificar erro imediato se status nao for sucesso;
- para streaming, ainda nao considerar TTFT.

#### T5: primeiro token util

No primeiro evento SSE ou chunk semantico que contenha delta util de resposta:

- capturar `first_token_mono`;
- calcular `ttft_ms = first_token_mono - start_mono`;
- emitir `FirstTokenObserved` para metricas de baixa latencia, se a fila permitir;
- encaminhar o chunk ao cliente imediatamente.

Definicao importante: TTFT deve ser medido quando o primeiro conteudo util e observado e encaminhado, nao quando os headers chegam.

#### T6: chunks intermediarios

Para cada chunk:

- incrementar `bytes_out`;
- alimentar parser SSE incremental;
- detectar usage quando o provedor envia metadados;
- atualizar contador de output tokens se o evento trouxer delta tokenizado;
- nunca persistir por chunk no banco relacional.

Eventos por chunk so devem ser usados para debug com sampling agressivo.

#### T7: usage/final event

Quando o provedor envia usage final:

- capturar `input_tokens` oficial se disponivel;
- capturar `output_tokens` oficial se disponivel;
- capturar `total_tokens`;
- marcar fonte da contagem: `provider`, `local_tokenizer`, `estimate` ou `unknown`.

Se nao houver usage final, usar contador incremental ou estimativa.

#### T8: fim normal do stream

Ao finalizar:

- capturar `completed_at`;
- capturar `end_mono`;
- calcular `total_latency_ms`;
- finalizar status;
- montar `TelemetryEvent` compacto e owned;
- tentar enfileirar em canal/fila assincrona;
- liberar buffers e contexto;
- encerrar resposta do cliente.

O enfileiramento deve ser non-blocking ou bounded com timeout extremamente curto.

#### T9: erro, cancelamento ou disconnect

Em erro:

- classificar causa: upstream timeout, upstream reset, provider 5xx, client disconnected, admission rejected, quota exceeded, parse error;
- capturar ponto de falha;
- cancelar upstream se cliente desconectou;
- montar evento final;
- tentar enfileirar;
- liberar recursos.

### 8.5 Acumulador local de telemetria

Cada request deve ter um acumulador pequeno, preferencialmente alocado no contexto pooled:

```text
TelemetryAccumulator
  request_id
  tenant_id
  route_id
  provider_id
  model_requested
  model_resolved
  started_at
  start_mono
  first_token_mono
  end_mono
  bytes_in
  bytes_out
  input_tokens
  output_tokens
  status
  error_class
  flags
```

Esse acumulador nao deve conter:

- prompt completo;
- mensagens completas;
- chunks de resposta;
- headers sensiveis;
- corpo bruto;
- ponteiros para buffers que voltarao ao pool.

### 8.6 Evento final owned

O evento que sai para background precisa ser independente da memoria do request:

```text
TelemetryFinalEvent
  request_id: fixed/owned
  tenant_id: fixed/owned
  route_id: small enum/string id
  provider_id: small enum/string id
  model: interned id or compact string
  timestamps: numeric
  durations: numeric
  counters: numeric
  status: enum
  error: enum
  token_source: enum
```

Nao enviar slices do payload para a fila.

### 8.7 Fila de telemetria

Requisitos:

- bounded;
- preferencialmente MPSC por shard;
- baixa contencao;
- metricas de profundidade;
- politica explicita quando cheia;
- batch drain por worker;
- shutdown com drenagem limitada por tempo.

Politicas quando cheia:

| Tipo de evento | Politica padrao |
|---|---|
| metricas agregaveis | drop + contador |
| eventos de debug por chunk | drop agressivo |
| billing final | tentar WAL local ou backpressure curto |
| auditoria obrigatoria | fila dedicada com limite e alerta |

### 8.8 Worker de telemetria

O worker deve:

- ler eventos em lote;
- compactar/enriquecer;
- aplicar sampling se necessario;
- escrever em sink analitico;
- fazer retry com backoff;
- expor lag, drops e erros;
- nunca chamar de volta o request context.

Sinks possiveis:

- ClickHouse para analitica de alta cardinalidade;
- Kafka/Redpanda/NATS para desacoplamento duravel;
- Postgres apenas para estado transacional pequeno ou consulta operacional;
- arquivos WAL locais para fallback temporario;
- OpenTelemetry exporter com batch processor.

### 8.9 Persistencia por lote

Writes devem ser agrupados por:

- quantidade de eventos;
- tamanho em bytes;
- janela maxima de tempo;
- chave de particionamento.

Exemplo:

```text
flush quando:
  batch_size >= 5_000 eventos
  OU batch_bytes >= 4 MiB
  OU batch_age >= 1s
```

Valores exatos devem ser calibrados em benchmark, mas o principio e evitar uma escrita por request.

### 8.10 Billing e consistencia

Para billing, precisamos separar:

- **medicao no caminho quente**: rapida, compacta, nao bloqueante;
- **consolidacao financeira**: background, idempotente, reconciliavel.

Eventos finais devem ter chave idempotente:

```text
idempotency_key = request_id + provider_attempt + terminal_state
```

O sink deve tolerar duplicatas ou usar deduplicacao por chave.

### 8.11 Privacidade e seguranca

Por padrao, telemetria nao deve persistir prompt nem completion. Quando auditoria exigir conteudo:

- usar politica explicita por tenant;
- criptografar;
- mascarar PII quando possivel;
- aplicar retencao curta;
- separar sink de auditoria do sink de metricas;
- nunca colocar conteudo sensivel em logs de erro comuns.

### 8.12 Criterios de aceitacao da telemetria

- Nenhuma escrita sincrona em Postgres no caminho do request.
- Evento final e enfileirado sem depender de flush externo.
- Fila cheia nao derruba o plano de dados em modo padrao.
- TTFT e extraido no primeiro delta util.
- Tokens oficiais substituem estimativas quando disponiveis.
- Client disconnect cancela upstream e gera evento terminal.
- Lag e drops de telemetria sao observaveis.

---

## 9. Fluxo de Requisicao Fim a Fim

```text
1. Client conecta ao gateway.
2. Listener aceita conexao e cria contexto pooled.
3. Auth identifica tenant.
4. Parser seletivo extrai campos minimos.
5. Router escolhe provider/model/rota.
6. Translator decide pass-through ou transcodificacao.
7. Upstream pool entrega stream/conexao disponivel.
8. Request e enviado ao provedor.
9. Headers upstream chegam.
10. Stream pump inicia leitura incremental.
11. Primeiro delta util define TTFT.
12. Chunks sao encaminhados ao cliente e observados lateralmente.
13. Usage final e capturado se existir.
14. Stream termina ou falha.
15. Evento final owned e publicado na fila de telemetria.
16. Buffers/contextos sao limpos e devolvidos ao pool.
17. Worker de telemetria persiste por lote.
```

---

## 10. Contratos de Alta Disponibilidade

### 10.1 Health checks

Separar:

- liveness: processo esta vivo;
- readiness: aceita trafego;
- upstream readiness: provedores disponiveis;
- telemetry readiness: sinks saudaveis, sem impedir data path por padrao.

### 10.2 Circuit breakers

Por provider/model/region:

- abrir por taxa de erro;
- abrir por timeout;
- abrir por P99 acima de limite;
- half-open com amostragem controlada;
- fallback somente quando semantica permitir.

### 10.3 Shutdown gracioso

Ao receber sinal:

1. parar de aceitar novas conexoes;
2. manter streams existentes ate deadline de drenagem;
3. cancelar streams que excederem deadline;
4. publicar eventos finais de cancelamento;
5. drenar fila de telemetria por tempo limitado;
6. fechar processo.

### 10.4 Degradacao controlada

Sob pressao:

- reduzir sampling de debug;
- descartar eventos nao criticos;
- limitar novos tenants de baixa prioridade;
- aumentar rejeicoes por admissao;
- preservar streams ja aceitos quando possivel;
- evitar retry storm.

---

## 11. Observabilidade do Proprio Gateway

Metricas obrigatorias:

- requests por rota/provedor/modelo;
- inflight requests;
- inflight streams;
- TTFT p50/p95/p99;
- latencia total p50/p95/p99;
- bytes in/out;
- tokens in/out;
- erros por classe;
- client disconnects;
- upstream resets;
- pool hits/misses;
- conexoes upstream abertas;
- streams por conexao;
- fila de telemetria: profundidade, lag, drops;
- tempo de flush de telemetria;
- alocacoes por request;
- GC pause em Go ou allocator pressure em Rust;
- CPU por plano: data/control/telemetry.

Logs devem ser amostrados e estruturados. O log por request completo nao deve ser padrao em alta carga.

---

## 12. Benchmarks e Validacao

### 12.1 Benchmarks obrigatorios

Reproduzir os cenarios internos:

- conexao direta ao vLLM;
- gateway pass-through sem telemetria;
- gateway pass-through com telemetria async;
- gateway com transcodificacao OpenAI -> Anthropic;
- gateway com SSE intensivo;
- gateway com client disconnect;
- gateway com sink de telemetria lento;
- gateway com fila de telemetria cheia;
- gateway com provedor degradado.

### 12.2 Metricas de sucesso

Comparar contra baseline:

| Metrica | Meta arquitetural |
|---|---|
| Vazao pass-through | aproximar-se do vLLM direto, com overhead pequeno e estavel |
| TTFT | overhead minimo sobre upstream |
| P99 | crescimento sublinear sob concorrencia |
| Alocacoes/request | baixo e previsivel |
| Escritas DB/request no caminho quente | zero |
| Drops de telemetria critica | zero ou reconciliado por WAL |
| Drops de debug | permitido e mensurado |

### 12.3 Perfis

Coletar:

- CPU profile;
- heap profile;
- allocation profile;
- mutex/block profile;
- scheduler trace;
- flamegraph do stream path;
- latencia por etapa;
- syscall profile quando aplicavel.

Em Go, usar:

- `pprof`;
- `go tool trace`;
- escape analysis;
- metricas de GC.

Em Rust, usar:

- `tokio-console`;
- `perf`;
- `flamegraph`;
- allocator profiling;
- tracing spans com sampling.

---

## 13. Decisao Go vs Rust

### 13.1 Go

Vantagens:

- produtividade alta;
- runtime maduro para rede;
- goroutines simples;
- ecossistema HTTP robusto;
- boa experiencia operacional;
- pprof excelente.

Riscos:

- GC sob alta taxa de alocacao;
- escapes para heap dificeis de perceber;
- channels mal dimensionados geram bloqueios;
- `net/http` pode impor custos abstratos em cenarios extremos.

Go e adequado se o design for disciplinado em pooling, parse seletivo e telemetria async.

### 13.2 Rust

Vantagens:

- controle de memoria;
- ausencia de GC;
- `Bytes`/ownership favorecem zero-copy realista;
- performance previsivel;
- enums e tipos fortes para protocolo.

Riscos:

- complexidade de lifetimes em parsers borrowed;
- maior custo de desenvolvimento;
- risco de clones defensivos se o time nao dominar ownership;
- ecossistema HTTP/3 e integracoes podem exigir mais trabalho.

Rust e adequado se a prioridade maxima for previsibilidade de latencia, controle de memoria e performance sustentada.

### 13.3 Invariante independente de linguagem

A linguagem nao salva uma arquitetura que escreve em banco no caminho critico. Go bem desenhado deve superar Rust mal desenhado; Rust bem desenhado deve oferecer teto de performance superior. O criterio principal e preservar:

- streaming incremental;
- pouca alocacao;
- telemetria desacoplada;
- backpressure;
- pooling;
- multiplexacao.

---

## 14. Riscos e Mitigacoes

| Risco | Impacto | Mitigacao |
|---|---|---|
| Transcodificacao complexa entre provedores | bugs semanticos | adapters testados por contrato |
| Fila de telemetria cheia | perda de metrica | sampling, WAL, alertas |
| Uso excessivo de pool | vazamento de dados entre requests | reset rigoroso e testes |
| Buffer grande retido | memoria elevada | classes de tamanho e descarte |
| Retry storm | amplificacao de falha | retry budget e circuit breaker |
| HTTP/2 stream limit mal calibrado | P99 instavel | auto-tuning e metricas |
| Token counting caro | latencia extra | contagem async/estimada quando necessario |
| Logs verbosos | I/O collapse | sampling e batch |
| Client disconnect ignorado | desperdicio upstream | cancellation propagation |

---

## 15. Padroes de Projeto Recomendados

### 15.1 Adapter por provedor

Cada provedor deve implementar:

```text
ProviderAdapter
  encode_request(canonical_view) -> upstream_body
  decode_stream(bytes) -> stream_events_for_metering
  map_error(upstream_error) -> gateway_error
  extract_usage(event) -> usage_delta
```

O adapter nao deve persistir metrica nem acessar banco.

### 15.2 Strategy para roteamento

Roteamento deve ser plugavel:

- menor latencia;
- menor custo;
- tenant affinity;
- weighted round-robin;
- failover regional;
- modelo equivalente.

### 15.3 Circuit breaker por chave composta

Chave:

```text
provider_id + region + model_family + auth_profile
```

### 15.4 Producer-consumer para telemetria

Plano de dados e producer. Worker de telemetria e consumer. A fila e a unica fronteira.

### 15.5 Bulkhead

Separar recursos por classe:

- provedores;
- tenants grandes;
- telemetria critica;
- telemetria debug;
- modelos caros.

Um tenant ou provedor degradado nao deve consumir todo o gateway.

---

## 16. Checklist de Implementacao Inicial

1. Criar listener HTTP com suporte a streaming.
2. Implementar request context pooled.
3. Implementar parser seletivo para campos minimos.
4. Implementar roteador simples provider/model.
5. Implementar upstream connection pool com keep-alive.
6. Implementar pass-through OpenAI-compatible.
7. Implementar stream pump SSE.
8. Implementar metering tap incremental.
9. Implementar telemetry accumulator.
10. Implementar fila bounded de telemetria.
11. Implementar worker batch para sink inicial.
12. Implementar backpressure de admissao.
13. Implementar cancellation por client disconnect.
14. Implementar metricas internas.
15. Rodar benchmark contra vLLM direto e gateway.

---

## 17. Definicao de Pronto

A fundacao da camada proxy base sera considerada pronta quando:

- o caminho OpenAI-compatible pass-through funcionar sem parse completo;
- SSE for encaminhado incrementalmente;
- TTFT for medido corretamente;
- telemetria final for enfileirada sem flush sincrono;
- Postgres nao aparecer no flamegraph do caminho de request;
- buffers e contexts forem reaproveitados com reset seguro;
- HTTP/2 multiplexar streams upstream quando suportado;
- o gateway lidar com 500 requisicoes concorrentes sem degradacao equivalente ao baseline de -75,6%;
- testes de carga demonstrarem que a fila de telemetria lenta nao derruba o plano de dados;
- client disconnect cancelar upstream;
- erros e timeouts gerarem evento terminal.

---

## 18. Tese Arquitetural

A degradacao de 75,6% observada no gateway tradicional nao e um acidente de implementacao isolado; ela e consequencia de colocar persistencia, objetos dinamicos, logging e tracing sincronos no mesmo caminho que deveria apenas mover bytes e preservar semantica de protocolo.

O gateway de alta performance deve ser desenhado como um proxy L7 especializado em IA:

- move bytes rapidamente;
- interpreta apenas o necessario;
- transforma somente quando inevitavel;
- mede sem bloquear;
- persiste fora do caminho quente;
- controla concorrencia antes do colapso;
- usa memoria com disciplina de runtime/ownership.

Essa arquitetura transforma o gateway de um gargalo serializante em uma camada de controle e observabilidade que preserva a eficiencia do motor de inferencia.

---

## 19. Validacao Pratica da Tese (Stage 2 - TC-020)

Em 28 de junho de 2026, os artefatos locais existentes foram reconciliados para preparar o alpha v1. A leitura publica correta e:

1. **Gateway com telemetria ativa**:
   - Baseline direto: 20.282,05 req/s.
   - Gateway: 17.831,39 req/s.
   - Degradacao calculada contra o direto: **12,08%**.
   - Esse valor fica dentro do gate local/virtualizado de Stage 2, que aceita degradacao menor que 15% em WSL2/localhost.

2. **Gateway com logs em `/dev/null`**:
   - Gateway: 18.014,34 req/s.
   - Degradacao calculada contra o direto: **11,18%**.
   - O numero de **1,01%** mede apenas a diferenca entre gateway com logs em `/dev/null` e gateway com telemetria ativa. Ele nao deve ser publicado como overhead do gateway contra a conexao direta.

3. **Limite da evidencia antiga**:
   - Os JSONs brutos antigos registram RPS, P95 e erro HTTP, mas nao registram P99.
   - O resumo oficial fica em `benchmarks/alpha-v1-benchmark-summary.md`.
   - Para claim publico completo, rodar novamente o k6 com `handleSummary()` atualizado para registrar P95/P99, ambiente, commit e comandos.

Esses resultados sustentam que a telemetria permanece desacoplada do caminho critico em modo local de laboratorio, mas nao devem ser usados para afirmar overhead de 1,01% contra o baseline direto.
