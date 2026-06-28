# Multi-Agent Handoff

Status: protocolo de passagem de trabalho entre Codex, Gemini, DeepSeek e outros agentes

---

## 1. Por Que Existe

Agentes diferentes tem estilos diferentes. Este protocolo evita que um agente:

- reinvente a arquitetura;
- ignore contexto;
- pule testes;
- prometa performance sem medir;
- deixe o proximo agente perdido.

---

## 2. Handoff Obrigatorio

Todo agente deve terminar com:

```text
Status: verde/amarelo/vermelho
Card ou etapa:
Objetivo:
Arquivos alterados:
Comandos rodados:
Resultados:
O que ficou pendente:
Riscos:
Proximo passo sugerido:
```

Se nao rodou teste:

```text
Nao rodei <comando> porque <motivo>.
```

Nunca escrever:

```text
testes devem passar
```

Escrever:

```text
testes passaram / testes falharam / testes nao foram rodados
```

---

## 3. Context Pack Para Novo Agente

Antes de trabalhar, envie ao agente:

```text
Leia AGENTS.md.
Leia docs/implementation-playbook.md.
Leia docs/agent-task-cards.md.
Leia docs/validation-gates.md.
Execute somente o card <ID>.
Nao altere licenca.
Nao adicione banco sincrono no hot path.
No final, responda no formato de handoff.
```

---

## 4. Quando Usar Cada Agente

### Codex

Melhor para:

- editar repo local;
- rodar comandos;
- criar arquivos;
- ajustar CI;
- validar em shell;
- coordenar task cards.

### Gemini

Melhor para:

- revisar documentacao longa;
- encontrar contradicoes conceituais;
- criticar estrategia;
- avaliar README e narrativa;
- sugerir simplificacao de DX.

### DeepSeek

Melhor para:

- raciocinio algoritmico;
- revisar trechos Rust;
- sugerir otimizacoes;
- isolar bugs;
- comparar implementacoes.

Observacao:

```text
Mesmo quando outro agente sugerir codigo, Codex deve aplicar, compilar e validar localmente quando possivel.
```

---

## 5. Controle de Escopo

Um agente recebe um card. Se encontrar problema fora do card:

```text
registre como risco ou proximo card;
nao refatore tudo;
nao mude arquitetura sem ADR.
```

---

## 6. Handoff Para Benchmark

Formato:

```text
Benchmark:
Ambiente:
Commit:
Comando direto:
Resultado direto:
Comando gateway:
Resultado gateway:
Degradacao RPS:
P99 direto:
P99 gateway:
Status:
Interpretacao:
```

---

## 7. Handoff Para Bug

Formato:

```text
Bug:
Sintoma:
Como reproduzir:
Hipotese:
Arquivo suspeito:
Fix aplicado:
Teste:
Resultado:
```

---

## 8. Handoff Para Arquitetura

Formato:

```text
Decisao:
Alternativas:
Trade-off:
Impacto em hot path:
Impacto em telemetria:
Precisa ADR: sim/nao
Recomendacao:
```

---

## 9. Checklist de Recebimento

Quando um agente receber trabalho de outro:

1. leia o ultimo handoff;
2. rode `git status`;
3. confira arquivos alterados;
4. rode validacao minima se possivel;
5. continue do ponto pendente;
6. nao recomece do zero sem motivo.
