# Benchmarks

Resultados brutos devem ser salvos em `benchmarks/results/`.

Esse diretorio e ignorado por Git para evitar commitar artefatos grandes. Para publicar um resultado, crie um resumo em Markdown com:

```text
data
commit
hardware
sistema operacional
comando
resultado direto
resultado gateway
degradacao
P99
observacoes
```

## Baseline Direto

```bash
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
```

## Gateway

```bash
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

## Gate Inicial

```text
degradacao_rps_percent < 2
P99 aproximadamente flat
http_req_failed < 0.1%
```

## vLLM Native Parity

Quando vLLM estiver rodando localmente:

```bash
k6 run -e TARGET_URL=http://localhost:8000/v1/chat/completions k6/proxy-vs-direct.js
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

Publicar somente se:

```text
RPS gateway >= 98% do vLLM direto
P99 gateway aproximadamente flat
TTFT overhead documentado
```
