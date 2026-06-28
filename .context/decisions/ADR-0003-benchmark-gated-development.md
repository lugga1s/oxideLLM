# ADR-0003: Desenvolvimento Gated por Benchmark

Status: aceito  
Data: 2026-06-28

## Contexto

O projeto tem proposta publica baseada em performance. Sem benchmark, o risco e virar marketing tecnico sem prova.

## Decisao

Toda etapa relevante tera gate objetivo em `docs/validation-gates.md`.

## Racional

Gates protegem o projeto contra:

- claims exagerados;
- features antes da base;
- regressao de performance;
- complexidade sem evidencia;
- agentes avancando sem validacao.

## Consequencias

Positivas:

- progresso mensuravel;
- feedback claro para engenheiro nao tecnico;
- README mais confiavel;
- agentes trabalham com autonomia.

Negativas:

- implementacao pode parecer mais lenta;
- exige disciplina de ambiente e artefatos;
- alguns resultados locais podem precisar repeticao em Linux dedicado.
