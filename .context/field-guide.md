# Guia de Campo Para o Engenheiro de Contexto

Este guia existe para voce acompanhar o projeto sem precisar ser especialista em Rust, redes ou sistemas distribuidos.

## O Que Estamos Construindo

Um gateway local de IA. Ele fica entre sua aplicacao e provedores como OpenAI, Anthropic, Ollama ou vLLM.

Ele faz tres coisas:

1. recebe a requisicao;
2. encaminha a resposta da IA em streaming;
3. registra metadados sem atrasar o usuario.

## Como Saber se Estamos Indo Bem

O projeto so esta indo bem quando passa nos gates de `docs/validation-gates.md`.

O indicador mais importante no inicio:

```text
O gateway adiciona menos de 2% de perda de RPS em relacao ao mock direto.
```

Traducao: se chamar o mock direto gera 1000 req/s, chamar atraves do gateway deve gerar pelo menos 980 req/s no teste pass-through.

## O Que Pedir Para Um Agente

Bons pedidos:

```text
Implemente a Stage 2 e rode os gates.
Compare proxy vs direto usando k6 e salve os resultados.
Crie uma ADR se mudar a biblioteca HTTP.
Explique em semaforo o status da etapa atual.
```

Pedidos perigosos:

```text
Adicione dashboard agora.
Adicione 100 provedores antes do benchmark.
Coloque Postgres para salvar cada request.
Publique no README que e 10x mais rapido sem benchmark.
```

## Semaforo

Verde:

```text
gate passou com evidencia.
```

Amarelo:

```text
funciona, mas ainda nao temos prova suficiente.
```

Vermelho:

```text
gate falhou; nao avancar.
```

## Perguntas de Controle

Use estas perguntas para revisar qualquer entrega:

1. Qual etapa foi atacada?
2. Qual comando foi rodado?
3. Qual foi o numero antes e depois?
4. O gate passou?
5. O agente atualizou docs se mudou arquitetura?
6. Existe risco escondido?

## Direcao Certa

O projeto deve caminhar nesta ordem:

```text
documentacao e CI
mock e benchmark direto
proxy SSE simples
telemetria em memoria
profiling de alocacao
micro-batching local
upstream real Ollama/vLLM
provedores adicionais
README publico com benchmark
```

Nao inverter essa ordem sem motivo forte.

## Como Acompanhar a Execucao Agora

Use estes tres arquivos:

```text
docs/implementation-playbook.md
docs/agent-task-cards.md
docs/multi-agent-handoff.md
```

Se outro agente entrar no projeto, entregue um card pequeno e exija handoff no formato padrao.
