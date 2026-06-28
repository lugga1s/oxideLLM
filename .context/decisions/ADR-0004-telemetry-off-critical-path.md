# ADR-0004: Telemetria Fora do Caminho Critico

Status: aceito  
Data: 2026-06-28

## Contexto

O gargalo principal documentado e a mistura de proxy, logs, traces e persistencia no caminho da resposta.

## Decisao

Telemetria sera publicada em estrutura bounded de memoria e consumida por worker em background. Persistencia nao bloqueia cliente em modo padrao.

## Racional

Isso preserva:

- TTFT;
- P99;
- throughput;
- isolamento entre rede e analitica.

## Consequencias

Positivas:

- cliente nao espera disco;
- sistema suporta sink lento;
- micro-batching reduz custo de I/O.

Negativas:

- eventos podem ser descartados se fila encher, conforme politica;
- billing/auditoria exigem caminho especial;
- shutdown precisa drenar com prazo.

## Politica Inicial

```text
debug events: drop quando cheio
final usage events: tentar preservar, registrar drop se falhar
auditoria obrigatoria: futuro modo separado
```
