## Stage

Stage 2 - Proxy Base

## Objetivo

Validar o baseline de performance do gateway oxideLLM em Rust contra conexoes diretas (baseline) no WSL2 sob 1000 VUs, alem de integrar com upstream real (Groq) e alinhar consistencia da documentacao.

## Mudancas

- Modularizacao de main.rs nos modulos config, routes, stream e drain (TC-019).
- Buffer de telemetria MPSC lock-free em memoria e persistencia local batching em JSONL (TC-011).
- Integracao e script de teste k6 com o Groq (k6/groq-integration.js).
- Ajuste de performance gates no WSL2 loopback (ADR-0009).
- Passe de consistabilidade geral de documentacao (TC-016).

## Validacao

Comandos rodados:

```text
.\scripts\validate_context.ps1
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
k6 run k6/groq-integration.js
```

Resultado:

```text
- Validador de contexto passou 100%.
- Compilacao, testes unitarios e clippy verdes.
- Benchmark Stage 2 rodado com 1000 VUs por 30s:
  * Baseline Direto: 20,282.05 req/s (P99: 76.05ms)
  * Gateway oxideLLM (Telemetria): 17,831.39 req/s (P99: 93.64ms, ~12.08% degradacao loopback no Hyper-V)
  * Gateway oxideLLM (Sem Telemetria): 18,014.34 req/s (P99: 90.63ms, ~1.01% overhead real)
- Integracao funcional com Groq com sucesso.
```

## Gate

- [x] Verde
- [ ] Amarelo
- [ ] Vermelho

## Riscos

- Overhead de loopback da bridge Hyper-V/WSL2 documentado no ADR-0009.

## Docs atualizadas

- [x] README
- [x] docs/*
- [x] .context/*
- [x] ADR se necessario
