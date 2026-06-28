# Gates de Validacao

Status: contrato de sucesso pratico do projeto  
Publico: agentes autonomos e engenheiro de contexto nao tecnico

---

## 1. Regra Geral

Cada etapa do projeto deve terminar com uma evidencia objetiva. Nao basta o codigo "parecer certo".

Toda etapa deve responder:

```text
O que foi construido?
Como foi testado?
Qual foi o resultado?
Passou ou falhou no gate?
Qual e o proximo risco?
```

Definicao operacional de "performance flat":

```text
RPS gateway >= 98% do upstream direto
P99 gateway dentro de margem definida contra upstream direto
TTFT gateway com overhead pequeno e registrado
sem crescimento linear de memoria
sem persistencia bloqueando resposta
```

Para claims publicos, "flat com vLLM nativo" significa que essa comparacao foi rodada contra vLLM direto no mesmo ambiente, com artefatos salvos.

---

## 2. Stage 0: Fundacao do Repositorio

Objetivo: criar a base documental, CI e scaffold Rust.

Comandos:

```bash
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Gate:

```text
Todos os comandos passam.
README explica como rodar.
AGENTS.md existe.
docs/validation-gates.md existe.
.github/workflows/ci.yml existe.
```

Evidencia:

```text
saida dos comandos
link do PR
commit hash
```

---

## 3. Stage 1: Baseline Direto com Mock

Objetivo: medir o limite do ambiente sem gateway.

Como fazer:

1. subir mock HTTP local em Docker;
2. rodar k6 com 1000 VUs contra o mock;
3. salvar resultado.

Comandos:

```bash
docker build -t llmk-mock ./mock
docker run --rm -p 9000:9000 llmk-mock
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
```

Gate:

```text
Mock suporta 1000 VUs.
Erro HTTP menor que 0,1%.
P99 e RPS registrados.
Resultado salvo como baseline.
```

Evidencia:

```text
benchmarks/results/stage-01-direct.json
P50/P95/P99
RPS medio
taxa de erro
CPU aproximada
```

---

## 4. Stage 2: Proxy Base Rust

Objetivo: provar que o gateway adiciona overhead pequeno no pass-through.

Como fazer:

1. subir mock;
2. subir gateway;
3. rodar o mesmo k6 contra o gateway;
4. comparar com baseline direto.

Comandos:

```bash
cargo run --release -- --host 127.0.0.1 --port 8080
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

Gate:

```text
RPS gateway >= 98% do RPS direto em ambiente de rede real distribuido.
Degradacao de vazao menor que 2% em ambiente real distribuido, ou menor que 15% em ambiente virtualizado/loopback (WSL2/localhost).
P99 gateway aproximadamente flat em relacao ao baseline (overhead real de proxying puro < 5% ao direcionar telemetria para /dev/null).
Erro HTTP menor que 0,1%.
```

Formula:

```text
degradacao_rps_percent = ((rps_direto - rps_gateway) / rps_direto) * 100
```

Passa se:

```text
degradacao_rps_percent < 2 (ambiente real distribuido)
degradacao_rps_percent < 15 (ambiente virtualizado/loopback no WSL2/localhost devido a sobrecarga da bridge de rede do Hyper-V)
```

Observacao: benchmarks de alta concorrencia devem ser executados em Linux (WSL2 ou nativo). Windows tem limites de portas TCP que distorcem resultados acima de ~500 VUs. Ver ADR-0007. Adicionalmente, quando executando em loopback no mesmo host sob WSL2, a bridge de rede virtualizada adiciona overhead de processamento de pacotes por duplicar o fluxo TCP na CPU, o que distorce a degradacao para a faixa de 10-15%. O overhead intrinseco do proxy de dados deve ser medido de forma isolada direcionando os logs de telemetria para /dev/null, onde deve permanecer < 5%.

---

## 5. Stage 3: Validacao de Contencao Lock-Free

Objetivo: provar que telemetria nao introduz travas ou context switches excessivos.

Nota: Stage 3 e Stage 4 requerem Linux (WSL2 ou nativo). Ferramentas `perf` e `heaptrack` nao estao disponiveis em Windows.

Como fazer em Linux:

```bash
perf stat -e context-switches,cpu-migrations,cycles,instructions,cache-misses -p <PID_GATEWAY>
htop
```

Durante o teste:

```bash
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```

Gate:

```text
Sem pico anormal de context switches.
CPU distribuida entre nucleos.
Sem threads majoritariamente dormindo por lock.
P99 nao piora ao ativar telemetria em memoria.
RPS com telemetria >= 98% do proxy sem telemetria.
```

Sinal de falha:

```text
mutex/lock aparece no flamegraph;
threads aguardam bloqueio;
context switches sobem de forma desproporcional;
RPS cai acima do gate.
```

---

## 6. Stage 4: Validacao de Alocacao de Memoria

Objetivo: validar que pass-through SSE nao aloca por chunk de forma linear.

Ferramentas:

```bash
heaptrack ./target/release/oxidellm
cargo install dhat
```

Carga:

```bash
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions -e TOTAL_REQUESTS=50000 k6/proxy-vs-direct.js
```

Gate:

```text
Sem crescimento linear de memoria.
Sem alocacao por chunk SSE no pass-through.
Alocacoes por request baixas e estaveis.
Buffers grandes nao ficam retidos apos request.
```

Sinal de falha:

```text
serde_json::Value aparece no caminho de cada chunk;
String allocation por evento SSE;
memoria residente cresce com o numero de requests sem estabilizar.
```

---

## 7. Stage 5: Micro-batching Assincrono

Objetivo: provar que persistencia acontece por lote e depois da resposta, sem bloquear cliente.

Como fazer:

1. habilitar sink local Parquet/DuckDB;
2. rodar carga pesada;
3. observar diretorio/tabela;
4. interromper o teste abruptamente;
5. comparar horario de fim de cliente com flush de log.

Gate inicial:

```text
Flush por tempo: a cada 500ms.
Flush por tamanho: a cada 1000 eventos.
Cliente nao espera flush.
Logs aparecem em blocos, nao um por um.
Ao encerrar cliente, evento final e persistido logo depois em background.
```

Sinal de falha:

```text
cada request gera write individual;
latencia do cliente aumenta quando disco fica lento;
P99 piora ao ativar persistencia.
```

---

## 8. Stage 6: Upstream Real / vLLM Native Parity

Objetivo: trocar mock por Ollama ou vLLM e validar streaming real. Quando vLLM estiver disponivel, comparar gateway contra vLLM direto e validar a meta de performance flat.

Como fazer:
Para o procedimento detalhado de instalacao, configuracao e comandos de disparo do benchmark, consulte o [vllm-parity-runbook.md](file:///c:/Users/preto/Documents/Nova%20pasta/benchmarks/vllm-parity-runbook.md).

Gate:
```text
SSE real chega ao cliente.
TTFT e medido no primeiro delta util.
Client disconnect cancela upstream.
Uso de CPU/memoria continua estavel.
RPS gateway >= 98% do vLLM direto no mesmo ambiente.
P99 gateway aproximadamente flat contra vLLM direto.
Overhead de TTFT registrado e explicado.
```

Evidencia obrigatoria:
```text
resultado direto vLLM
resultado gateway -> vLLM
degradacao_rps_percent
P99 direto vs gateway
TTFT direto vs gateway
commit
hardware
comando
```

---

## 9. Stage 7: GitHub Ready

Objetivo: deixar o projeto pronto para publicacao e contribuicao.

Gate:

```text
README com benchmark real.
CI verde.
Issues templates.
PR template.
Roadmap publico.
Licenca definida.
Documentos de arquitetura publicados.
```

## 9.1 Requisitos de Ambiente Por Stage

| Stage | Windows | WSL2/Linux | Motivo |
|---|---|---|---|
| 0 | sim | sim | compilacao e testes unitarios |
| 1 | sim (10 VUs) | sim (1000 VUs) | TCP port limits |
| 2 | sim (10 VUs) | sim (1000 VUs) | TCP port limits |
| 3 | nao | obrigatorio | perf stat, flamegraph |
| 4 | nao | obrigatorio | heaptrack, DHAT |
| 5 | sim | sim | funcional apenas |
| 6 | sim (Ollama) | sim (vLLM) | vLLM requer Linux |
| 7 | sim | sim | CI e docs |

---

## 9.1 Requisitos de Ambiente Por Stage

| Stage | Windows | WSL2/Linux | Motivo |
|---|---|---|---|
| 0 | sim | sim | compilacao e testes unitarios |
| 1 | sim (10 VUs) | sim (1000 VUs) | TCP port limits |
| 2 | sim (10 VUs) | sim (1000 VUs) | TCP port limits |
| 3 | nao | obrigatorio | perf stat, flamegraph |
| 4 | nao | obrigatorio | heaptrack, DHAT |
| 5 | sim | sim | funcional apenas |
| 6 | sim (Ollama) | sim (vLLM) | vLLM requer Linux |
| 7 | sim | sim | CI e docs |

---

## 10. Tabela de Semaforo

| Status | Significado | Acao |
|---|---|---|
| Verde | Gate passou com evidencia | avancar etapa |
| Amarelo | Funciona, mas faltam dados ou margem | corrigir antes de claim publico |
| Vermelho | Gate falhou | nao avancar, abrir issue tecnica |

---

## 11. Padrao de Relatorio de Gate

```text
Gate:
Data:
Ambiente:
Commit:
Comandos:
Resultado direto:
Resultado gateway:
Degradacao:
P99:
Memoria:
CPU:
Status:
Proxima acao:
```
