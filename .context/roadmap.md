# Roadmap Tecnico

Status: plano incremental orientado por gates

## Stage 0: Fundacao

Objetivo:

- docs;
- scaffold Rust;
- CI;
- AGENTS.md;
- base de contexto.
- playbook operacional;
- task cards;
- handoff multi-agente.

Saida:

- repo organizado;
- comandos de validacao definidos.

## Stage 1: Baseline Direto

Objetivo:

- mock HTTP/SSE em Docker;
- k6 contra mock direto;
- primeiro baseline local.

Saida:

- resultado direto salvo;
- P99/RPS conhecidos.

## Stage 2: Proxy Base

Objetivo:

- gateway Rust aceita `/v1/chat/completions`;
- encaminha/gera SSE sem telemetria pesada;
- comparacao gateway vs direto.

Saida:

- degradacao de RPS menor que 2%;
- P99 flat.

## Stage 3: Telemetria em Memoria

Objetivo:

- evento final compacto;
- fila bounded;
- contador de drops;
- zero flush no caminho critico.

Saida:

- benchmark com telemetria ligada;
- queda menor que 2% contra proxy sem telemetria.

## Stage 4: Profiling de Memoria

Objetivo:

- heaptrack/DHAT;
- garantir que pass-through nao aloca por chunk.

Saida:

- relatorio de heap;
- hotspots documentados.

## Stage 5: Micro-batching Local

Objetivo:

- worker background;
- flush por tempo/tamanho;
- sink local inicial.

Saida:

- logs aparecem em blocos;
- cliente nao espera disco.

## Stage 6: Upstream Real

Objetivo:

- Ollama ou vLLM;
- streaming real;
- cancelamento por disconnect;
- TTFT real.

Saida:

- demo local reproduzivel.

## Stage 7: GitHub Ready

Objetivo:

- README publico;
- benchmark real;
- templates;
- roadmap;
- release inicial.

Saida:

- projeto pronto para estrelas, issues e contribuicoes.
