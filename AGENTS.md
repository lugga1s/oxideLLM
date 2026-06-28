# AGENTS.md

Este arquivo define como agentes de IA devem trabalhar neste repositorio. Ele e obrigatorio para qualquer agente que leia, implemente, teste, documente ou publique mudancas.

## Missao do Projeto

Construir um gateway/proxy de LLMs em Rust que preserve o desempenho do upstream, processe SSE de forma eficiente e desacople telemetria/persistencia do caminho critico.

## Regra Suprema

Nenhum agente deve afirmar sucesso de performance sem evidencia pratica:

- comando executado;
- ambiente;
- resultado medido;
- arquivo de saida ou resumo numerico;
- comparacao com gate definido em `docs/validation-gates.md`.

## Documentos Obrigatorios

Antes de alterar codigo, leia:

1. `.context/project-manifest.md`
2. `.context/bottlenecks.md`
3. `docs/implementation-playbook.md`
4. `docs/agent-execution-system.md`
5. `docs/agent-task-cards.md`
6. `docs/multi-agent-handoff.md`
7. `docs/agent-quality-scorecard.md`
8. `docs/review-gates.md`
9. `docs/architecture.md`
10. `docs/validation-gates.md`
11. `docs/production-ritual.md`
12. este `AGENTS.md`

Se a tarefa envolver GitHub, leia tambem:

13. `docs/github-workflow.md`

Se a ferramenta local estiver ausente, leia:

14. `docs/tooling-setup.md`

## Autonomia Permitida

Agentes podem, sem pedir permissao:

- criar branches locais;
- criar testes;
- rodar `cargo fmt`;
- rodar `cargo check`;
- rodar `cargo test`;
- rodar `cargo clippy`;
- criar benchmarks k6;
- atualizar docs de contexto;
- criar ADRs em `.context/decisions/`;
- preparar PR draft quando houver remoto GitHub configurado.

Agentes nao devem, sem confirmacao explicita:

- fazer force-push;
- apagar historico Git;
- remover arquivos de contexto;
- trocar Rust por Go;
- adicionar banco relacional no caminho critico;
- transformar logs de request em escrita sincrona;
- publicar segredo, API key, prompt sensivel ou token.

## Forma de Trabalho

Cada tarefa deve seguir este ciclo:

1. Ler contexto relevante.
2. Escolher um card em `docs/agent-task-cards.md`.
3. Declarar objetivo tecnico.
4. Fazer mudanca pequena.
5. Rodar validacao local proporcional.
6. Atualizar docs se a arquitetura mudou.
7. Registrar decisao relevante como ADR.
8. Resumir resultado no formato de `docs/multi-agent-handoff.md`.

## Feedback Para Engenheiro Nao Tecnico

O feedback deve usar linguagem clara:

```text
Status: verde/amarelo/vermelho
O que foi feito:
O que foi testado:
Resultado:
Proximo passo:
Risco atual:
```

Evite jargoes sem explicar. Quando usar termos como P99, TTFT, CAS, MPSC, heap ou SSE, explique em uma frase.

## Padrao de Branches

```text
main
feature/stage-01-proxy-baseline
feature/stage-02-sse-passthrough
feature/stage-03-telemetry-ring
feature/stage-04-microbatch
fix/<descricao-curta>
docs/<descricao-curta>
bench/<descricao-curta>
```

## Padrao de Commits

Use commits pequenos:

```text
docs: add validation gates
feat: add sse mock endpoint
bench: add k6 proxy baseline
test: cover telemetry queue overflow
ci: add rust checks
```

## Validacoes Minimas Antes de PR

Quando Rust estiver instalado:

```bash
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Quando k6 estiver instalado:

```bash
k6 run k6/proxy-vs-direct.js
```

Se uma ferramenta nao existir no ambiente, registre o bloqueio no resumo e nao finja que o teste passou.

## Invariantes Tecnicos

- Telemetria nao bloqueia resposta do cliente.
- Fila de telemetria e bounded.
- Payloads grandes nao sao convertidos para strings sem necessidade.
- SSE e encaminhado incrementalmente.
- Client disconnect cancela upstream.
- Benchmarks comparam direto vs gateway.
- Claims publicos precisam de resultado reproduzivel.
- Licenca do projeto e AGPL-3.0-or-later, salvo decisao formal em ADR.

## Quando Criar ADR

Crie um ADR quando:

- trocar biblioteca central;
- alterar arquitetura de telemetria;
- mudar gate de sucesso;
- adicionar sink de persistencia;
- mudar protocolo publico;
- decidir entre Hyper/Axum/Pingora;
- aceitar degradacao de performance acima do gate.
- alterar licenca ou modelo de distribuicao.
