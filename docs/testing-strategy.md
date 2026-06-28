# Estrategia de Testes

Status: guia de validacao automatica e manual

---

## 1. Piramide de Testes

```text
unit tests
contract tests
integration tests
load tests
profiling tests
release benchmarks
```

## 2. Unit Tests

Cobrir:

- fila de telemetria cheia;
- drain por lote;
- parse de SSE;
- calculo de latencia;
- politica de backpressure;
- mapeamento de erros.

Comando:

```bash
cargo test --all
```

## 3. Contract Tests

Cobrir:

- OpenAI-compatible request;
- SSE `[DONE]`;
- usage final;
- erro antes de stream;
- erro durante stream;
- client disconnect.

Fixtures:

```text
tests/fixtures/openai/
tests/fixtures/ollama/
tests/fixtures/anthropic/
```

## 4. Integration Tests

Subir:

- mock upstream;
- gateway;
- cliente HTTP;
- asserts de status e SSE.

## 5. Load Tests

Ferramenta:

```text
k6
```

Cenarios:

```text
100 VUs smoke
1000 VUs gate
50000 total requests memory gate
sink lento telemetry gate
client disconnect storm
```

## 6. Profiling

CPU:

```bash
perf record -g ./target/release/litellm-killer
```

Context switches:

```bash
perf stat -e context-switches,cpu-migrations,cycles,instructions -p <PID>
```

Heap:

```bash
heaptrack ./target/release/litellm-killer
```

## 7. CI

Checks obrigatorios:

```bash
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Benchmarks pesados podem rodar manualmente ou em workflow separado.

## 8. Criterio de Regressao

Uma mudanca e regressao se:

- aumenta P99 acima do gate;
- reduz RPS mais que 2% no pass-through;
- introduz alocacao por chunk;
- bloqueia cliente por persistencia;
- remove backpressure;
- torna fila de telemetria unbounded.
