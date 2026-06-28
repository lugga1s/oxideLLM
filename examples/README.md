# Configuration Examples

Este diretorio contem exemplos de configuracao TOML para rodar o oxideLLM em
ambiente local.

## Arquivos

- `basic_config.toml`: configuracao minima para Ollama local.
- `multi_provider.toml`: configuracao com multiplos upstreams ordenados por
  prioridade para failover.
- `config.toml.example`: exemplo comentado com todos os campos principais.
- `.env.example`: variaveis de ambiente basicas.

## Ollama Local

Suba ou prepare o Ollama:

```bash
ollama pull tinyllama
```

Inicie o gateway com o exemplo minimo:

```bash
cargo run -- --config examples/basic_config.toml
```

Teste streaming OpenAI-compatible:

```bash
curl -N -X POST http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"tinyllama","messages":[{"role":"user","content":"Ola!"}],"stream":true}'
```

## Multi-provider e Failover

O exemplo `multi_provider.toml` tenta provedores em ordem crescente de
`priority`. Se um upstream estiver inativo ou retornar erro recuperavel, o
gateway pode tentar o proximo upstream saudavel.

```bash
cargo run -- --config examples/multi_provider.toml
```

Para testar localmente, mantenha apenas os upstreams que voce realmente tem
rodando ou ajuste as URLs. O worker de saude usa `health_path` para marcar cada
upstream como saudavel ou indisponivel.

## Ordem de Precedencia

As configuracoes seguem esta ordem:

1. argumentos de CLI;
2. variaveis de ambiente `LLMK_*`;
3. arquivo TOML passado por `--config`;
4. `config.toml` na raiz, quando existir;
5. defaults internos.

## Telemetria

Os exemplos gravam eventos em `telemetry_events.jsonl` por micro-batching. Essa
persistencia roda em background e nao deve bloquear a resposta do cliente.
