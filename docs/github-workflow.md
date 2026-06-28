# Workflow GitHub

Status: guia para agentes criarem branches, PRs, CI e releases

---

## 1. Premissas

O repositorio deve operar com branch principal protegida. Agentes trabalham por PR, nao diretamente em `main`.

Branch principal:

```text
main
```

Branches de trabalho:

```text
feature/<stage-ou-feature>
fix/<bug>
docs/<tema>
bench/<experimento>
ci/<mudanca>
```

---

## 2. Criar Branch

```bash
git checkout main
git pull --ff-only
git checkout -b feature/stage-02-sse-passthrough
```

---

## 3. Validar Antes de Commit

```bash
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Se a mudanca envolver benchmark:

```bash
k6 run k6/proxy-vs-direct.js
```

---

## 4. Commit

```bash
git add .
git commit -m "feat: add sse passthrough baseline"
```

Use tipos:

```text
feat
fix
docs
test
bench
ci
refactor
perf
chore
```

---

## 5. Push e PR

```bash
git push -u origin feature/stage-02-sse-passthrough
gh pr create --draft --title "Stage 2: SSE passthrough baseline" --body-file .github/PULL_REQUEST_TEMPLATE.md
```

Se `gh` nao estiver autenticado:

```bash
gh auth login
```

Agente deve reportar o bloqueio ao engenheiro de contexto se nao houver remoto ou autenticacao.

---

## 6. Protecao de Branch

Configurar no GitHub:

```text
Require pull request before merging
Require status checks to pass
Require branches to be up to date
Require linear history
Restrict force pushes
```

Status checks obrigatorios:

```text
context checks
fmt
check
test
clippy
```

Benchmarks longos nao precisam bloquear todo PR no inicio, mas PRs de performance devem anexar resultado.

---

## 7. Template de PR

Todo PR deve responder:

```text
Stage:
Objetivo:
Mudancas:
Validacao:
Resultado:
Riscos:
Docs atualizadas:
```

---

## 8. Releases

Versoes iniciais:

```text
0.1.0 foundation
0.2.0 sse proxy baseline
0.3.0 telemetry ring
0.4.0 microbatch local
0.5.0 ollama/vllm real upstream
1.0.0 stable local gateway
```

Release so deve incluir benchmark se houver artefato reproduzivel.
