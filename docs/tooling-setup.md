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
WSL2: benchmarks de alta concorrencia e profiling (ver ADR-0007)
```

## 2. Estado Detectado Neste Ambiente

Ultima verificacao local (2026-06-28):

```text
rustc: disponivel (1.87+)
cargo: disponivel
k6: ausente (instalar via winget ou WSL2)
docker: disponivel
git: disponivel
gh: disponivel
python: disponivel
wsl2: verificar com wsl --status
ollama: verificar com ollama --version
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

Para benchmarks de alta concorrencia, instalar k6 dentro do WSL2 (ver secao 6.1).

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

## 6.1 WSL2 (Obrigatorio Para Benchmarks e Profiling)

WSL2 e necessario para benchmarks de alta concorrencia (100+ VUs) e profiling (perf, heaptrack). Ver ADR-0007 para contexto da decisao.

Instalar WSL2 (PowerShell como administrador):

```powershell
wsl --install
```

Reiniciar o computador apos a instalacao. Depois, dentro do WSL2:

```bash
# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup component add rustfmt clippy

# Instalar k6
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg \
  --keyserver hkp://keyserver.ubuntu.com:80 \
  --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D68
echo 'deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main' \
  | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6

# Instalar ferramentas de profiling
sudo apt-get install linux-tools-generic heaptrack
```

O projeto pode ser acessado diretamente de dentro do WSL2:

```bash
cd /mnt/c/Users/preto/Documents/Nova\ pasta
cargo build --release
k6 run k6/proxy-vs-direct.js
```

## 6.2 Ollama (Opcional Para Teste Com IA Real)

Ollama permite testar o gateway com um modelo de IA real local (TC-021, Sessao 7 do playbook).

```powershell
winget install Ollama.Ollama
ollama pull tinyllama
```

Validar:

```powershell
Invoke-RestMethod -Uri http://localhost:11434/v1/chat/completions -Method POST -ContentType "application/json" -Body '{"model":"tinyllama","messages":[{"role":"user","content":"hi"}],"stream":true}'
```

## 7. Primeiro Check Completo

Depois de instalar tudo:

```powershell
.\scripts\validate_context.ps1
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo build --manifest-path mock/Cargo.toml --release
```

Para benchmark completo (dentro do WSL2):

```bash
cd /mnt/c/Users/preto/Documents/Nova\ pasta
cargo build --release
cargo build --manifest-path mock/Cargo.toml --release
# Terminal 1: mock/target/release/oxidellm-mock
# Terminal 2: target/release/oxidellm
# Terminal 3:
k6 run -e TARGET_URL=http://localhost:9000/v1/chat/completions k6/proxy-vs-direct.js
k6 run -e TARGET_URL=http://localhost:8080/v1/chat/completions k6/proxy-vs-direct.js
```
