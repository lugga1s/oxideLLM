# Tooling Setup

Status: guia de instalacao de ferramentas locais

---

## 1. Ferramentas Obrigatorias

```text
Rust toolchain: rustc, cargo, rustfmt, clippy
k6: load testing
Docker: mock/upstream local
Git: versionamento
GitHub CLI: PRs e automacao
```

## 2. Estado Detectado Neste Ambiente

Ultima verificacao local:

```text
rustc: ausente
cargo: ausente
k6: ausente
docker: disponivel
git: disponivel
gh: disponivel
python: disponivel
```

## 3. Instalar Rust

Fonte oficial:

```text
https://rustup.rs/
```

Windows PowerShell:

```powershell
winget install Rustlang.Rustup
rustup default stable
rustup component add rustfmt clippy
```

Validar:

```powershell
rustc --version
cargo --version
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

## 4. Instalar k6

Fonte oficial:

```text
https://grafana.com/docs/k6/latest/set-up/install-k6/
```

Windows PowerShell:

```powershell
winget install k6.k6
k6 version
```

## 5. Docker

Validar:

```powershell
docker version
docker compose version
```

## 6. GitHub CLI

Validar:

```powershell
gh --version
gh auth status
```

Se nao autenticado:

```powershell
gh auth login
```

## 7. Primeiro Check Completo

Depois de instalar tudo:

```powershell
.\scripts\validate_context.ps1
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
docker build -t llmk-mock ./mock
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
```
