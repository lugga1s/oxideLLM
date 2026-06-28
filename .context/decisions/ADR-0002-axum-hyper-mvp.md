# ADR-0002: Axum/Hyper no MVP

Status: aceito  
Data: 2026-06-28

## Contexto

O projeto precisa de uma base HTTP em Rust. As opcoes consideradas foram:

- Axum sobre Hyper;
- Hyper direto;
- Pingora;
- framework custom.

## Decisao

Usaremos Axum sobre Hyper no MVP.

## Racional

Axum acelera a construcao do vertical slice:

- roteamento simples;
- integracao com Tower;
- base Hyper;
- ergonomia boa para endpoints iniciais;
- suficiente para validar SSE, telemetria e benchmarks.

Pingora sera reavaliado quando o projeto exigir recursos mais profundos de proxy L7.

## Consequencias

Positivas:

- MVP mais rapido;
- codigo mais acessivel para contribuidores;
- CI e testes mais simples.

Negativas:

- pode existir overhead acima de Hyper/Pingora puro;
- alguns controles finos de proxy podem exigir refatoracao.

## Gate de Reavaliacao

Reavaliar se:

- degradacao de RPS no Stage 2 for maior que 2%;
- P99 piorar sem causa simples;
- connection pooling/protocolos exigirem controle indisponivel em Axum.
