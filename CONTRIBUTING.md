# Contributing

Obrigado por contribuir. Este projeto e benchmark-first: mudancas relevantes precisam ser validadas por testes ou medicoes.

## Antes de Comecar

Leia:

- `README.md`
- `AGENTS.md`
- `docs/architecture.md`
- `docs/validation-gates.md`

## Setup

```bash
rustup show
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

## PRs

Todo PR deve:

- ter escopo pequeno;
- explicar o stage;
- listar comandos rodados;
- informar se algum gate foi afetado;
- atualizar docs quando muda arquitetura.

## Performance Claims

Nao adicione claims de performance sem:

- comando;
- ambiente;
- commit;
- resultado bruto;
- comparacao contra baseline.

## Licenca

Ao contribuir, voce concorda que sua contribuicao sera licenciada sob **AGPL-3.0-or-later**, a mesma licenca do projeto.

## Codigo

Evite:

- escrita sincrona em banco no caminho critico;
- fila sem limite;
- `serde_json::Value` por chunk SSE;
- clone de payload grande;
- task por token;
- logs verbosos por request sob carga.
