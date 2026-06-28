# Benchmarks

Resultados brutos devem ser salvos em `benchmarks/results/`.

Esse diretorio e ignorado por Git para evitar commitar artefatos grandes. Para publicar um resultado, crie um resumo em Markdown versionavel com:

```text
data
commit
hardware
sistema operacional
comando
resultado direto
resultado gateway
degradacao
P95
P99
observacoes
```

## Resumos Oficiais

- `alpha-v1-benchmark-summary.md`: reconciliacao oficial dos artefatos existentes para preparacao do alpha v1.
- `stage-02-wsl2-ext4-20260628-summary.md`: rodada Stage 2 completa em WSL2, a partir do filesystem Linux, com 1000 VUs por 30s.

## Script Automatizado WSL2

Para rodar Stage 2 com mock, gateway e k6 no WSL2:

```bash
bash scripts/run_stage2_benchmark_wsl.sh
```

O script compila os binarios em release, sobe o mock na porta 9000, sobe o gateway na porta 8080, roda o benchmark direto e depois via gateway, e salva tres JSONs em `benchmarks/results/`.

Para resultados mais limpos, rode a partir do filesystem Linux do WSL, por exemplo `~/oxidellm`, nao a partir de `/mnt/c/...`. O WSL emite um aviso de desempenho para cargas intensivas de E/S em unidades do Windows, e isso pode distorcer o benchmark.

## Baseline Direto

```bash
k6 run -e RUN_LABEL=stage-02-direct \
  -e TARGET_URL=http://localhost:9000/v1/chat/completions \
  -e SUMMARY_PATH=benchmarks/results/stage-02-direct-summary.json \
  k6/proxy-vs-direct.js
```

## Gateway

```bash
k6 run -e RUN_LABEL=stage-02-gateway \
  -e TARGET_URL=http://localhost:8080/v1/chat/completions \
  -e SUMMARY_PATH=benchmarks/results/stage-02-gateway-summary.json \
  k6/proxy-vs-direct.js
```

O script `k6/proxy-vs-direct.js` usa `handleSummary()` para salvar JSON com P95 e P99. Defina `SUMMARY_PATH` para evitar sobrescrever artefatos entre a rodada direta e a rodada via gateway.

## Gate Inicial Em Rede Real

```text
degradacao_rps_percent < 2
P99 aproximadamente flat
http_req_failed < 0.1%
```

## Gate Local WSL2/localhost

```text
degradacao_rps_percent < 15
P99 registrado e comparado
http_req_failed < 0.1%
```

## vLLM Native Parity

Para detalhes de configuracao de ambiente, instalacao do vLLM, execucao do servidor e disparos de carga comparativos, consulte o guia passo a passo em [vllm-parity-runbook.md](vllm-parity-runbook.md).

Quando o vLLM estiver rodando localmente, execute os comandos descritos no runbook para coletar os resultados direto vs gateway.

Publicar resultados somente se:

- RPS gateway >= 98% do vLLM direto;
- P99 gateway aproximadamente flat;
- TTFT overhead documentado e explicado;
- artefatos salvos e resumo Markdown versionavel criado.
