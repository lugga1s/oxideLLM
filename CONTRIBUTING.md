# Contributing

Obrigado por contribuir com o oxideLLM. Este projeto e benchmark-first:
mudancas relevantes precisam ser validadas por testes ou medicoes, e nenhuma
claim de performance deve ser publicada sem evidencia pratica.

## Antes de Comecar

Leia estes arquivos para entender o contrato tecnico do repositorio:

- `README.md`
- `AGENTS.md`
- `docs/architecture.md`
- `docs/implementation-playbook.md`
- `docs/validation-gates.md`
- `docs/github-workflow.md`

## Setup do Ambiente

Ferramentas esperadas:

- Rust toolchain estavel, gerenciado por `rustup`
- Git
- GitHub CLI (`gh`) para abrir PRs
- k6 para benchmarks quando a mudanca tocar performance
- Docker para mocks e cenarios de validacao local

Comandos uteis:

```bash
rustup show
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Para rodar o gateway localmente:

```bash
cargo run -- --host 127.0.0.1 --port 8080
```

Para usar um exemplo pronto:

```bash
cargo run -- --config examples/basic_config.toml
```

## Branches e Commits

Use branches pequenas e descritivas:

```text
feature/<descricao-curta>
fix/<descricao-curta>
docs/<descricao-curta>
bench/<descricao-curta>
ci/<descricao-curta>
```

Use commits pequenos com prefixos convencionais:

```text
feat: add sse passthrough baseline
fix: handle upstream timeout
docs: add validation notes
test: cover telemetry overflow
ci: add dependency audit
```

## Checklist de PR

Antes de abrir um PR, confirme:

- [ ] O escopo esta pequeno e descrito no PR.
- [ ] `cargo fmt --check` foi executado.
- [ ] `cargo check --all-targets` foi executado.
- [ ] `cargo test --all` foi executado.
- [ ] `cargo clippy --all-targets -- -D warnings` foi executado.
- [ ] `cargo audit` foi executado quando disponivel.
- [ ] Benchmarks k6 foram rodados quando a mudanca afetou performance.
- [ ] O resultado cita comandos, ambiente e status do gate relevante.
- [ ] Docs ou ADRs foram atualizados quando a arquitetura mudou.
- [ ] Nenhum segredo, token, prompt sensivel ou API key foi commitado.

Se uma ferramenta estiver ausente, registre o bloqueio no PR em vez de assumir
que a validacao passou.

## Bugs e Propostas

Ao reportar um bug, inclua:

- versao ou commit;
- sistema operacional;
- comando executado;
- resultado esperado;
- resultado observado;
- logs relevantes sem segredos.

Para propostas de arquitetura, explique o impacto no caminho critico do
gateway, especialmente em SSE, telemetria e alocacoes de memoria.

## Performance Claims

Nao adicione claims de performance sem:

- comando executado;
- ambiente;
- commit;
- resultado bruto ou resumo numerico;
- comparacao contra baseline direto;
- referencia ao gate em `docs/validation-gates.md`.

Termos rapidos:

- P99: 99% das requisicoes foram mais rapidas que esse tempo.
- TTFT: tempo ate o primeiro token util aparecer.
- SSE: formato de streaming usado para enviar tokens incrementalmente.

## Codigo

Evite:

- escrita sincrona em banco no caminho critico;
- fila sem limite;
- `serde_json::Value` por chunk SSE;
- clone de payload grande;
- task por token;
- logs verbosos por request sob carga.

Prefira:

- streams de bytes incrementais;
- filas bounded;
- telemetria assincrona;
- testes pequenos e focados;
- exemplos reproduziveis.

## Conduta

Ainda nao ha um `CODE_OF_CONDUCT.md` separado. Enquanto isso, contribua com
respeito, foco tecnico e linguagem clara. Discussao dura sobre codigo e bem
vinda; ataque pessoal nao e.

## Licenca

Ao contribuir, voce concorda que sua contribuicao sera licenciada sob
**AGPL-3.0-or-later**, a mesma licenca do projeto.
