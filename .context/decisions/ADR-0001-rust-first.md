# ADR-0001: Rust First

Status: aceito  
Data: 2026-06-28

## Contexto

O projeto precisa construir um gateway de IA com baixa latencia, streaming eficiente, controle de memoria e telemetria desacoplada. Go e Rust eram opcoes iniciais.

## Decisao

Usaremos Rust como linguagem principal do projeto.

## Racional

Rust oferece:

- ausencia de GC;
- ownership para controlar buffers;
- seguranca contra data races em codigo seguro;
- bom ecossistema async com Tokio;
- boa base para parsers zero-copy;
- forte narrativa tecnica para performance.

Go continua sendo uma alternativa viavel, mas Rust se alinha melhor ao objetivo de previsibilidade de P99 e controle de alocacoes.

## Consequencias

Positivas:

- maior controle sobre memoria;
- menor jitter associado a GC;
- diferenciacao tecnica forte;
- mais seguranca em concorrencia.

Negativas:

- curva de aprendizado maior;
- implementacao inicial mais lenta;
- mais cuidado com lifetimes e ownership;
- menor simplicidade para contribuidores iniciantes.

## Como Validar

Rust so justifica a escolha se os gates mostrarem:

- overhead de proxy pequeno;
- alocacoes controladas;
- P99 estavel;
- telemetria sem bloqueio.
