# ADR-0008: Troca de crossbeam-queue por tokio::sync::mpsc para Telemetria

Status: aceito  
Data: 2026-06-28

## Contexto

Durante o benchmark TC-020 (1000 VUs, ~21k RPS diretos), observamos degradacao severa do gateway (RPS caiu para ~7.8k, P99 piorou 5.6x). O perfil de logs mostrou milhares de contencoes de fila cheia na telemetria: `warn!("telemetry queue full while recording request completion")`.
A implementacao original de `telemetry.rs` utilizava `crossbeam_queue::ArrayQueue` e um worker de drain baseado em poll (loop de 100ms `tokio::time::interval`). Este modelo poll-based com timer era muito lento para esvaziar os eventos gerados (pico de >40k eventos por segundo). Quando a fila ficava cheia, milhares de threads competiam em `push()`, sofrendo contencao severa por operacoes CAS na ArrayQueue, e consequentemente gerando logs bloqueantes no hot path devido a trava interna do `tracing-subscriber`.

## Decisao

Trocar `crossbeam_queue::ArrayQueue` por canais delimitados (bounded) nativos do Tokio (`tokio::sync::mpsc::channel`).
Isto permite que o worker de drain se torne **reativo** (via `recv().await`), consumindo eventos imediatamente em vez de aguardar ticks fixos de tempo.

## Racional

O objetivo e que o envio de telemetria permaneca `non-blocking` (usando `try_send`).
O uso de canais MPSC do Tokio permite usar primitivas prontas do ecossistema assincrono.
Como Tokio ja e dependencia do projeto, o uso de `tokio::sync::mpsc` remove uma dependencia extra (`crossbeam-queue`) e padroniza as primitivas de concorrencia.
Adicionalmente, os `warn!()` no hot path por fila cheia sao substituidos por contadores silenciosos, pois a contencao no sistema de logging agravava a degradacao de performance.

## Consequencias

Positivas:
- Drain responde instantaneamente quando um evento chega, em vez de esperar 100ms, mitigando overflow da fila.
- Menos contencao CAS no envio quando a fila se aproxima do limite.
- Reducao de uma dependencia (crossbeam-queue).
- Remocao do overhead gerado pelos logs da fila cheia no path critico.

Negativas:
- Uso estrito do runtime assincrono do Tokio na ponta do receiver (o que ja era o caso para o drain_worker, entao o impacto arquitetural e nulo).
