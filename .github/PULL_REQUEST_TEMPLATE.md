## Stage

<!-- Indique o Stage atual da Sessao de desenvolvimento (ex: Stage 2 - Proxy Base, Stage 7 - GitHub Ready) -->
[Stage-X: Nome do Stage]

## Objetivo

<!-- Descreva de forma concisa o que este PR resolve e qual e o objetivo tecnico -->


## Mudancas

<!-- Descreva as mudancas introduzidas por este PR (ex: refatores, novas features, correcoes de bugs) -->
- 

## Validacao

<!-- Detalhe os testes e validacoes locais executados -->
Comandos rodados:

```bash
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
.\scripts\validate_context.ps1
```

Resultado:

```text
[Cole a saida resumida dos comandos ou os resultados de benchmarks]
```

## Gate

<!-- Indique o status do gate correspondente a este PR (consulte docs/validation-gates.md) -->
- [ ] Verde
- [ ] Amarelo
- [ ] Vermelho

## Riscos

<!-- Liste os riscos conhecidos ou limitacoes tecnicas remanescentes -->


## Docs atualizadas

- [ ] README
- [ ] docs/*
- [ ] .context/*
- [ ] ADR se necessario
