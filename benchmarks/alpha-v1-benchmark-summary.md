# Alpha v1 Benchmark Summary

Status: reconciliacao oficial dos artefatos existentes + smoke WSL2 com P95/P99
Data: 2026-06-28
Card: TC-016 Docs Consistency Pass + TC-015 Performance Review
Commit lido nesta sessao: `e07176d`

## Escopo

Este resumo reconcilia os artefatos locais existentes antes da publicacao alpha v1. Ele nao e uma nova execucao de benchmark.

Os artefatos brutos usados estao em `benchmarks/results/`, que e ignorado pelo Git para evitar commitar arquivos grandes. O resumo versionavel e este arquivo.

## Ambiente Registrado

Os documentos anteriores descrevem o teste como WSL2/localhost loopback com 1000 VUs por 30s. Os JSONs brutos do k6 nao gravaram hardware, sistema operacional, commit do binario nem P99. Portanto, o resultado abaixo e evidencia suficiente para reconciliar RPS/P95/erro, mas nao para claim publico completo de P99.

## Artefatos Usados

| Tipo | Arquivo bruto local |
|---|---|
| Direto contra mock Rust | `benchmarks/results/stage-02-direct-rust-mock-v2.json` |
| Gateway com telemetria ativa | `benchmarks/results/stage-02-gateway-rust-mock-v2.json` |
| Gateway com logs em `/dev/null` | `benchmarks/results/stage-02-gateway-rust-mock-v3.json` |

## Comandos Do Benchmark Original

Os comandos abaixo sao os comandos documentados/inferidos para os artefatos existentes:

```bash
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

Para novas execucoes, use `SUMMARY_PATH` para gerar JSON com P95 e P99 via `handleSummary()`:

```bash
k6 run -e RUN_LABEL=stage-02-direct \
  -e TARGET_URL=http://localhost:9000/v1/chat/completions \
  -e SUMMARY_PATH=benchmarks/results/stage-02-direct-summary.json \
  k6/proxy-vs-direct.js

k6 run -e RUN_LABEL=stage-02-gateway \
  -e TARGET_URL=http://localhost:8080/v1/chat/completions \
  -e SUMMARY_PATH=benchmarks/results/stage-02-gateway-summary.json \
  k6/proxy-vs-direct.js
```

## Resultados Reconciliados

| Caminho | RPS medio | P95 http_req_duration | P99 http_req_duration | HTTP error rate |
|---|---:|---:|---|---:|
| Direto contra mock Rust | 20.282,05 req/s | 59,64 ms | nao registrado | 0,00% |
| Gateway com telemetria ativa | 17.831,39 req/s | 74,40 ms | nao registrado | 0,00% |
| Gateway com logs em `/dev/null` | 18.014,34 req/s | 73,68 ms | nao registrado | 0,00% |

P95 significa que 95% das requisicoes foram mais rapidas que esse valor. P99 significa que 99% das requisicoes foram mais rapidas que esse valor; ele nao apareceu nos JSONs brutos antigos porque o script k6 ainda nao exportava `p(99)`.

## Calculo de Degradacao

Formula definida em `docs/validation-gates.md`:

```text
degradacao_rps_percent = ((rps_direto - rps_gateway) / rps_direto) * 100
```

Aplicacao:

```text
telemetria ativa:
((20282,05418823892 - 17831,39134393874) / 20282,05418823892) * 100 = 12,08%

logs em /dev/null:
((20282,05418823892 - 18014,344001210353) / 20282,05418823892) * 100 = 11,18%

diferenca entre gateway /dev/null e gateway com telemetria ativa:
((18014,344001210353 - 17831,39134393874) / 18014,344001210353) * 100 = 1,01%
```

## Interpretacao

O resultado de 12,08% de degradacao do gateway com telemetria ativa fica dentro do gate local/virtualizado de Stage 2, que permite degradacao menor que 15% em WSL2/localhost. O resultado de 11,18% com logs em `/dev/null` tambem fica dentro desse gate local.

O numero de 1,01% nao comprova overhead do gateway contra a conexao direta. Ele mede apenas a diferenca entre dois modos do gateway: telemetria ativa em disco versus logs direcionados para `/dev/null`.

## Smoke WSL2 Com `handleSummary()`

Esta sessao tambem executou um benchmark curto no WSL2 para validar que `k6/proxy-vs-direct.js` salva JSON com P95/P99.

Ambiente:

```text
Host: Windows com WSL2 Ubuntu
Rust: rustc 1.96.0
k6: v2.0.0 linux/amd64
Gateway telemetry log: /dev/null
Carga: 100 VUs por 10s
```

Servidores:

```bash
./mock/target/release/oxidellm-mock --host 127.0.0.1 --port 9000
./target/release/oxidellm --host 127.0.0.1 --port 8080 --upstream-base-url http://127.0.0.1:9000 --telemetry-log-path /dev/null
```

Comandos:

```bash
k6 run -e RUN_LABEL=alpha-v1-direct-wsl-smoke \
  -e VUS=100 \
  -e DURATION=10s \
  -e TARGET_URL=http://127.0.0.1:9000/v1/chat/completions \
  -e SUMMARY_PATH=benchmarks/results/alpha-v1-direct-wsl-smoke.json \
  k6/proxy-vs-direct.js

k6 run -e RUN_LABEL=alpha-v1-gateway-wsl-smoke \
  -e VUS=100 \
  -e DURATION=10s \
  -e TARGET_URL=http://127.0.0.1:8080/v1/chat/completions \
  -e SUMMARY_PATH=benchmarks/results/alpha-v1-gateway-wsl-smoke.json \
  k6/proxy-vs-direct.js
```

Resultados:

| Caminho | RPS medio | P95 http_req_duration | P99 http_req_duration | HTTP error rate |
|---|---:|---:|---:|---:|
| Direto contra mock Rust | 2.225,92 req/s | 48,11 ms | 48,66 ms | 0,00% |
| Gateway para mock Rust | 2.158,87 req/s | 48,28 ms | 50,14 ms | 0,00% |

Calculo:

```text
((2225,916562144127 - 2158,8680819665515) / 2225,916562144127) * 100 = 3,01%
```

Interpretacao:

```text
O smoke valida a geracao de artefatos JSON com P95/P99 e uma comparacao direta vs gateway.
Ele nao substitui o gate de alta concorrencia de 1000 VUs por 30s nem valida vLLM nativo.
```

## Status Para Publicacao

Status: amarelo para claim publico de performance.

Pode ser dito:

```text
Nos artefatos locais existentes, o gateway ficou 12,08% abaixo do mock direto com telemetria ativa em WSL2/localhost, dentro do gate local de <15%.
```

Nao deve ser dito ainda:

```text
O gateway tem apenas 1,01% de overhead real contra a conexao direta.
```

Para status verde de publicacao, rode novamente direto vs gateway com o `handleSummary()` atual, registre P95/P99, ambiente, commit, comandos e compare contra `docs/validation-gates.md`.
