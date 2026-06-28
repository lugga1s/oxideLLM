# Runbook de Paridade e Benchmark com vLLM

Este documento define o processo operacional para preparar o ambiente, implantar o vLLM, configurar o gateway `oxideLLM` e rodar os testes de carga comparativos para certificar que o gateway atende aos critérios de performance flat (degradação de RPS < 2% em rede física, ou < 15% em loopback WSL2).

---

## 1. Requisitos do Ambiente

Para executar este benchmark com fidelidade, você precisa de um ambiente Linux (nativo ou via WSL2) com acesso a uma GPU dedicada (NVIDIA).

* **OS**: Linux Ubuntu 22.04 LTS ou superior (nativo ou no WSL2).
* **Driver CUDA**: CUDA 12.1 ou superior instalado no host.
* **Python**: Versão 3.9 a 3.12.
* **Ferramentas**:
  * **Rust**: Toolchain ativo para compilação em modo release.
  * **k6**: Instalado no ambiente Linux para disparo de carga de concorrência.
  * **nvidia-container-toolkit**: Caso deseje executar o vLLM via Docker.

---

## 2. Preparação do Upstream (vLLM)

### Opção A: Instalação via Python Virtualenv (Recomendado)

1. Crie e ative um ambiente virtual Python:
   ```bash
   python3 -m venv venv-vllm
   source venv-vllm/bin/activate
   ```

2. Instale o vLLM (com suporte a CUDA do host):
   ```bash
   pip install --upgrade pip
   pip install vllm
   ```

3. Instalação alternativa para CPU-only (apenas para testes funcionais rápidos, sem validade de performance de carga):
   ```bash
   pip install vllm --extra-index-url https://download.pytorch.org/whl/cpu
   ```

### Opção B: Execução via Docker (Alternativa)

Se o Docker Desktop e o NVIDIA Container Toolkit estiverem ativos, você pode subir o vLLM com:
```bash
docker run --gpus all \
  -p 8000:8000 \
  --ipc=host \
  -v ~/.cache/huggingface:/root/.cache/huggingface \
  vllm/vllm-openai:latest \
  --model Qwen/Qwen2.5-0.5B-Instruct
```

---

## 3. Inicializando o Servidor vLLM

Para fins de benchmark ágil e reprodutível, utilizaremos o modelo leve **Qwen 2.5 0.5B Instruct** (ou Llama 3.2 1B Instruct).

Inicie o servidor vLLM:
```bash
vllm serve Qwen/Qwen2.5-0.5B-Instruct --port 8000 --max-model-len 2048
```

Valide se o servidor vLLM está respondendo diretamente em streaming:
```bash
curl -N -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "Qwen/Qwen2.5-0.5B-Instruct",
    "messages": [{"role": "user", "content": "Olá, me conte uma história curta."}],
    "stream": true
  }'
```

---

## 4. Inicializando o Gateway oxideLLM

1. Compile o gateway em modo release:
   ```bash
   cargo build --release
   ```

2. Execute o gateway apontando para a instância do vLLM como upstream:
   ```bash
   ./target/release/oxidellm \
     --host 127.0.0.1 \
     --port 8080 \
     --upstream-provider ollama \
     --upstream-base-url http://127.0.0.1:8000
   ```
   *(Nota: Usamos `--upstream-provider ollama` porque este driver opera em modo pass-through completo no corpo e no stream, o que é 100% compatível com a API do vLLM).*

3. Valide o acesso ao vLLM via gateway:
   ```bash
   curl -N -X POST http://localhost:8080/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{
       "model": "Qwen/Qwen2.5-0.5B-Instruct",
       "messages": [{"role": "user", "content": "Olá, me fale sobre a velocidade de luz."}],
       "stream": true
     }'
   ```

---

## 5. Execução do Benchmark (k6)

O script `k6/proxy-vs-direct.js` deve ser executado no mesmo ambiente de rede (localhost ou na mesma rede local se distribuído).

### Passo 1: Executar Carga Direta contra o vLLM (Baseline)
```bash
k6 run \
  -e TARGET_URL=http://localhost:8000/v1/chat/completions \
  -e MODEL_NAME=Qwen/Qwen2.5-0.5B-Instruct \
  --summary-export=benchmarks/results/vllm-direct-raw.json \
  k6/proxy-vs-direct.js
```

### Passo 2: Executar Carga contra o Gateway oxideLLM
```bash
k6 run \
  -e TARGET_URL=http://localhost:8080/v1/chat/completions \
  -e MODEL_NAME=Qwen/Qwen2.5-0.5B-Instruct \
  --summary-export=benchmarks/results/vllm-gateway-raw.json \
  k6/proxy-vs-direct.js
```

---

## 6. Análise de Resultados e Gates de Sucesso

Abra os arquivos de sumário exportados pelo k6 em `benchmarks/results/` para extrair os seguintes valores:
* **RPS Médio** (`http_reqs` rate)
* **P99 de Latência** (`http_req_duration` p99)
* **Overhead médio de TTFT** (capturado pelo gateway nos logs de telemetria em `telemetry_events.jsonl` ou inferido no k6).

### 6.1 Fórmulas de Validação

$$\text{degradacao\_rps\_percent} = \frac{\text{RPS}_{\text{direto}} - \text{RPS}_{\text{gateway}}}{\text{RPS}_{\text{direto}}} \times 100$$

### 6.2 Critérios de Aceitação (Gates)

* **Rede Real/Distribuída**:
  * $\text{degradacao\_rps\_percent} < 2\%$
* **Ambiente Virtualizado / Loopback (WSL2/Localhost)**:
  * $\text{degradacao\_rps\_percent} < 15\%$ (devido ao overhead da bridge TCP do hypervisor).
* **Taxa de Erro**:
  * $\text{http\_req\_failed} < 0.1\%$ para ambos os testes.
* **P99 Latency**:
  * P99 do gateway deve ser estatisticamente idêntica ou marginalmente superior ao direto ($< 5\%$ de acréscimo se telemetria estiver silenciada).

---

## 7. Registro do Benchmark

Sempre publique os dados no sumário em `benchmarks/README.md` sob a seção correspondente, registrando:
1. Data e Commit Hash.
2. Hardware (CPU, GPU, RAM).
3. Sistema Operacional (Windows 11, WSL2 Ubuntu, Linux Nativo).
4. Resultados numéricos (RPS e P99 direto vs gateway).
5. Percentual de degradação calculated.
