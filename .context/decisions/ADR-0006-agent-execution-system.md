# ADR-0006: Agent Execution System

Status: aceito  
Data: 2026-06-28

## Contexto

O projeto depende fortemente de agentes autonomos para implementar, testar, benchmarkar e publicar. Sem um sistema de execucao, agentes podem ampliar escopo, ignorar validacao ou divergir da arquitetura.

## Decisao

Adotar um sistema de execucao por agentes baseado em:

- instrucoes por ecossistema (`AGENTS.md`, `GEMINI.md`, `CLAUDE.md`, Copilot instructions);
- task cards pequenos;
- handoff padronizado;
- scorecard de qualidade;
- ledger de verificacao;
- prompts prontos por papel.

## Racional

Esse sistema melhora a viabilidade de conclusao porque transforma trabalho ambiguo em unidades verificaveis.

## Consequencias

Positivas:

- menos perda de contexto;
- menos claims falsos;
- mais facilidade para delegar;
- revisao objetiva de saidas;
- progresso mensuravel.

Negativas:

- mais arquivos de documentacao;
- exige disciplina para usar os cards;
- pode parecer pesado se aplicado a tarefas triviais.

## Regra de Uso

Para tarefas pequenas, use o sistema de forma leve. Para tarefas de performance, CI, GitHub ou arquitetura, use o sistema completo.
