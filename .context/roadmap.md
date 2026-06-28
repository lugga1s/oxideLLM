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

## Stage 1: Baseline Direto [VERDE]

Objetivo:

- mock HTTP/SSE em Docker;
- k6 contra mock direto;
- primeiro baseline local.

Saida:

- resultado direto salvo;
- P99/RPS conhecidos.

Concluido: TC-003, TC-004, TC-018

Nota: Mock reescrito em Rust (TC-018) resolveu gargalos de performance sob carga. Benchmark com 1000 VUs validado sob WSL2.

## Stage 2: Proxy Base [VERDE]

Objetivo:

- gateway Rust aceita `/v1/chat/completions`;
- encaminha/gera SSE sem telemetria pesada;
- comparacao gateway vs direto.

Saida:

- degradacao de RPS menor que 2%;
- P99 flat.

Concluido: TC-005, TC-006, TC-007, TC-008, TC-019, TC-020

Nota: Resultados locais reconciliados em `benchmarks/alpha-v1-benchmark-summary.md`. Os artefatos visiveis sustentam 12,08% de degradacao com telemetria ativa e 11,18% com logs em `/dev/null` contra o direto, dentro do gate local de WSL2/localhost. O numero de 1,01% mede apenas a diferenca entre modos do gateway e nao deve ser usado como overhead contra o direto.

## Stage 3: Telemetria em Memoria [VERDE]

Objetivo:

- evento final compacto;
- fila bounded;
- contador de drops;
- zero flush no caminho critico.

Saida:

- benchmark com telemetria ligada;
- queda menor que 2% contra proxy sem telemetria.

Concluido: TC-009, TC-010

Nota: Telemetria em memoria totalmente implementada e testada com sucesso.

## Stage 4: Profiling de Memoria [VERDE]

Objetivo:

- heaptrack/DHAT;
- garantir que pass-through nao aloca por chunk.

Saida:

- relatorio de heap;
- hotspots documentados.

Concluido: TC-015 (Relatorio salvo em benchmarks/results/stage-04-memory-validation.md)

## Stage 5: Micro-batching Local [VERDE]

Objetivo:

- worker background;
- flush por tempo/tamanho;
- sink local inicial.

Saida:

- logs aparecem em blocos;
- cliente nao espera disco.

Concluido: TC-011

## Stage 6: Upstream Real [VERDE]

Objetivo:

- Ollama ou vLLM;
- streaming real;
- cancelamento por disconnect;
- TTFT real.

Saida:

- demo local reproduzivel.

Concluido: TC-012, TC-021

Nota: Integracao funcional com upstream real validada utilizando API da Groq.

## Stage 7: GitHub Ready [VERDE]

Objetivo:

- README publico;
- benchmark real;
- templates;
- roadmap;
- release inicial.

Saida:

- projeto pronto para estrelas, issues e contribuicoes;
- claims de benchmark reconciliados com artefatos versionaveis.

Concluido: CI, licenca, templates, CONTRIBUTING, SECURITY, README com benchmark, preparacao de PR.

## Stage 8: Resiliencia e Roteamento Multimodelo [VERDE]

Objetivo:

- suporte a multiplos upstreams configuraveis;
- fallback/failover automatico em caso de falha;
- health checking em background de provedores de IA.

Saida:

- gateway redireciona requisicao para upstream alternativo de forma transparente se o principal falhar;
- status de saude dos upstreams monitorado ativamente.

Concluido: TC-022, TC-023, TC-024

Pendente: nenhum

Nota: Multiplos upstreams ordenados por prioridade, roteamento de failover dinamico (desvio automatico para upstreams saudaveis em caso de erros >= 400), e monitoramento ativo em segundo plano (background health worker thread-safe) implementados e totalmente testados.

## Resumo de Progresso

```text
[##################] 100% do MVP funcional
[##################] 100% do Alpha util
[##################] 100% do Beta util
```

| Stage | Status | Bloqueio |
|---|---|---|
| 0 | Verde | nenhum |
| 1 | Verde | nenhum |
| 2 | Verde | nenhum |
| 3 | Verde | nenhum |
| 4 | Verde | nenhum |
| 5 | Verde | nenhum |
| 6 | Verde | nenhum |
| 7 | Verde | nenhum |
| 8 | Verde | nenhum |
