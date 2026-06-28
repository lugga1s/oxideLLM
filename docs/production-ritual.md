# Rito de Producao

Status: guia operacional para engenheiro de contexto nao tecnico e agentes autonomos

---

## 1. Objetivo

Este documento define como o projeto deve evoluir sem caos. O engenheiro de contexto pode nao ser tecnico, entao os agentes precisam trabalhar com etapas claras, feedback legivel e criterios objetivos de sucesso.

O projeto nao avanca por opiniao. Avanca por gates.

---

## 2. Papeis

### Engenheiro de Contexto

Responsavel por:

- definir prioridade;
- aprovar direcao de produto;
- ler relatorios de semaforo;
- decidir se um risco e aceitavel;
- manter o norte estrategico.

Nao precisa:

- entender todos os detalhes de Rust;
- interpretar flamegraph sozinho;
- revisar cada linha de codigo;
- saber configurar CI manualmente.

### Agente Arquiteto

Responsavel por:

- manter blueprint;
- criar ADRs;
- revisar trade-offs;
- impedir violacao dos invariantes.

### Agente Implementador

Responsavel por:

- escrever codigo;
- criar testes;
- manter escopo pequeno;
- rodar validacoes.

### Agente Benchmark

Responsavel por:

- criar scripts k6;
- rodar baseline direto;
- rodar gateway;
- comparar resultados;
- salvar artefatos.

### Agente GitHub

Responsavel por:

- criar branch;
- abrir PR draft;
- preencher template;
- verificar CI;
- nao fazer force-push;
- nao mergear sem gate.

---

## 3. Ciclo Padrao de Uma Etapa

```text
1. Selecionar stage em docs/validation-gates.md.
2. Criar branch da etapa.
3. Implementar menor fatia funcional.
4. Rodar testes locais.
5. Rodar benchmark exigido.
6. Salvar evidencia.
7. Atualizar docs/ADR se necessario.
8. Abrir PR.
9. Resumir status em linguagem simples.
10. So avancar se gate estiver verde.
```

---

## 4. Como o Agente Deve Dar Feedback

Formato obrigatorio:

```text
Status: verde/amarelo/vermelho
Etapa:
O que mudou:
Como foi validado:
Resultado numerico:
Risco:
Proximo passo recomendado:
```

Exemplo:

```text
Status: amarelo
Etapa: Stage 2 - Proxy Base
O que mudou: gateway encaminha SSE mockado.
Como foi validado: cargo test passou; k6 rodou com 1000 VUs.
Resultado numerico: RPS caiu 3,4% contra direto; gate exige menos de 2%.
Risco: overhead extra no handler SSE.
Proximo passo recomendado: perfilar alocacoes no stream antes de seguir para telemetria.
```

---

## 5. Regras de Decisao

Se gate verde:

```text
merge permitido apos CI e revisao.
```

Se gate amarelo:

```text
nao publicar claim publico;
abrir issue tecnica;
permitir merge apenas se for infraestrutura interna e sem claim de performance.
```

Se gate vermelho:

```text
parar etapa;
nao avancar;
corrigir causa raiz.
```

---

## 6. Rito Diario de Agente

No inicio:

```text
git status
git pull --ff-only
ler docs relevantes
identificar stage atual
```

Durante:

```text
commits pequenos
testes frequentes
sem refatoracao nao relacionada
sem claims novos sem benchmark
```

No final:

```text
cargo fmt/check/test/clippy
benchmark se aplicavel
atualizar docs
resumir status
```

---

## 7. Rito de Benchmark Publico

Antes de colocar qualquer numero no README:

1. rodar baseline direto;
2. rodar gateway;
3. registrar hardware;
4. registrar sistema operacional;
5. registrar comando;
6. registrar commit;
7. salvar resultado bruto;
8. escrever interpretacao curta;
9. marcar se e local, laboratorio ou producao.

Sem isso, numero nao entra em material publico.

---

## 8. Rito de Incidente Tecnico

Quando um gate falhar:

```text
1. congelar novas features da etapa;
2. registrar falha;
3. criar hipotese;
4. coletar perfil;
5. corrigir;
6. repetir benchmark;
7. registrar aprendizado.
```

---

## 9. Linguagem Para Nao Tecnicos

Traducoes rapidas:

| Termo | Explicacao simples |
|---|---|
| P99 | 99% das requisicoes foram mais rapidas que esse tempo |
| TTFT | tempo ate o primeiro token aparecer |
| SSE | formato usado para enviar texto em streaming |
| heap | area de memoria onde alocacoes dinamicas acontecem |
| zero-copy | evitar duplicar dados na memoria |
| ring buffer | fila circular em memoria |
| lock-free | estrutura que evita travas tradicionais |
| micro-batching | gravar muitos logs juntos, nao um por um |

---

## 10. Norte de Produto

O projeto vence se conseguir demonstrar:

```text
instalacao simples
proxy rapido
telemetria local barata
benchmarks reproduziveis
documentacao clara
zero dependencia de SaaS para valor inicial
```
