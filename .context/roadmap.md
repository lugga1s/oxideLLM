# Roadmap Tecnico

Status: plano incremental orientado por gates  
Ultima atualizacao: 2026-06-28

## Stage 0: Fundacao [VERDE]

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

Concluido: TC-001, TC-002

## Stage 1: Baseline Direto [VERDE 10 VUs / AMARELO 1000 VUs]

Objetivo:

- mock HTTP/SSE em Docker;
- k6 contra mock direto;
- primeiro baseline local.

Saida:

- resultado direto salvo;
- P99/RPS conhecidos.

Concluido: TC-003, TC-004

Nota: mock Python saturava sob carga. Mock reescrito em Rust (TC-018) resolve este gargalo. Benchmark com 1000 VUs deve ser refeito em WSL2.

## Stage 2: Proxy Base [AMARELO]

Objetivo:

- gateway Rust aceita `/v1/chat/completions`;
- encaminha/gera SSE sem telemetria pesada;
- comparacao gateway vs direto.

Saida:

- degradacao de RPS menor que 2%;
- P99 flat.

Concluido: TC-005, TC-006, TC-007, TC-008, TC-019

Pendente: TC-020 (benchmark limpo em WSL2 com mock Rust)

Nota: resultados de 10 VUs mostram overhead de 0,03ms (excelente). Falta validar com 1000 VUs em WSL2 para gate verde definitivo.

## Stage 3: Telemetria em Memoria [AMARELO]

Objetivo:

- evento final compacto;
- fila bounded;
- contador de drops;
- zero flush no caminho critico.

Saida:

- benchmark com telemetria ligada;
- queda menor que 2% contra proxy sem telemetria.

Concluido: TC-009, TC-010 (codigo e testes prontos)

Pendente: profiling com perf stat (requer WSL2/Linux)

## Stage 4: Profiling de Memoria [NAO INICIADO]

Objetivo:

- heaptrack/DHAT;
- garantir que pass-through nao aloca por chunk.

Saida:

- relatorio de heap;
- hotspots documentados.

Requer: WSL2 ou Linux nativo (ver ADR-0007)

## Stage 5: Micro-batching Local [VERDE]

Objetivo:

- worker background;
- flush por tempo/tamanho;
- sink local inicial.

Saida:

- logs aparecem em blocos;
- cliente nao espera disco.

Concluido: TC-011

## Stage 6: Upstream Real [NAO INICIADO]

Objetivo:

- Ollama ou vLLM;
- streaming real;
- cancelamento por disconnect;
- TTFT real.

Saida:

- demo local reproduzivel.

Proximo: TC-021 (Ollama)

Nota: Ollama funciona em Windows. vLLM requer Linux.

## Stage 7: GitHub Ready [AMARELO]

Objetivo:

- README publico;
- benchmark real;
- templates;
- roadmap;
- release inicial.

Saida:

- projeto pronto para estrelas, issues e contribuicoes.

Concluido: CI, licenca, templates, CONTRIBUTING, SECURITY

Pendente: README com benchmark real, release binario, quickstart verificado

## Resumo de Progresso

```text
[##########--------] ~60% do MVP funcional
[######------------] ~35% do Alpha util
```

| Stage | Status | Bloqueio |
|---|---|---|
| 0 | Verde | nenhum |
| 1 | Verde/Amarelo | re-run com mock Rust |
| 2 | Amarelo | benchmark WSL2 |
| 3 | Amarelo | profiling WSL2 |
| 4 | Nao iniciado | WSL2 |
| 5 | Verde | nenhum |
| 6 | Nao iniciado | Ollama/vLLM |
| 7 | Amarelo | benchmark real |
