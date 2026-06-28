# Contratos de Protocolo

Status: especificacao inicial de compatibilidade LLM

---

## 1. Objetivo

Definir como o gateway deve tratar payloads e streams de provedores de IA sem transformar cada evento em objeto pesado.

O contrato principal do MVP e OpenAI-compatible:

```text
POST /v1/chat/completions
Content-Type: application/json
Accept: text/event-stream
```

---

## 2. OpenAI-compatible Chat Completions

Campos minimos de entrada:

```json
{
  "model": "string",
  "stream": true,
  "messages": [
    { "role": "user", "content": "string" }
  ]
}
```

Campos que o roteador pode extrair:

```text
model
stream
max_tokens
temperature
tools
```

Regra:

```text
Se o upstream aceita OpenAI-compatible, fazer pass-through sempre que politica permitir.
```

---

## 3. SSE

SSE e um stream de texto com eventos separados por linha em branco.

Exemplo:

```text
data: {"choices":[{"delta":{"content":"hello"}}]}

data: [DONE]

```

Parser deve reconhecer:

- `data:`;
- `event:`;
- comentarios iniciados por `:`;
- `\n\n`;
- `\r\n\r\n`;
- evento terminal `[DONE]`;
- fragmentos divididos entre buffers.

Parser nao deve:

- exigir que cada read de rede contenha evento completo;
- converter o stream inteiro para string;
- criar `serde_json::Value` para cada chunk se apenas bytes precisam ser encaminhados.

---

## 4. TTFT

TTFT significa time to first token.

Medir:

```text
ttft = momento do primeiro delta util - momento de entrada da request
```

Nao medir TTFT em:

- chegada de headers;
- criacao de conexao;
- primeiro byte vazio;
- keep-alive/comment SSE.

---

## 5. Tokens

Fonte de tokens deve ser marcada:

```text
provider
local_tokenizer
estimate
unknown
```

Prioridade:

1. usage oficial do provedor;
2. tokenizer local;
3. estimativa;
4. unknown.

Prompt/completion completos nao devem ser persistidos por padrao.

---

## 6. Anthropic

Anthropic usa Messages API com streaming baseado em eventos. Adaptacao OpenAI <-> Anthropic deve ser fase posterior.

Regra:

```text
Nao implementar transcodificacao Anthropic completa antes de passar nos gates de proxy OpenAI-compatible.
```

Quando implementar, criar testes de contrato para:

- system prompt;
- messages;
- tools;
- max_tokens;
- stream;
- event types;
- usage.

---

## 7. Ollama

Ollama oferece compatibilidade OpenAI em endpoints proprios. Para MVP local:

```text
Ollama OpenAI-compatible e o primeiro upstream real recomendado.
```

Motivo:

- roda localmente;
- reduz custo de teste;
- simplifica reproducibilidade;
- permite validar streaming real.

Integracao no Gateway:

- **Provedor**: `ollama`
- **Porta Padrao**: `11434`
- **Endpoint**: `/v1/chat/completions` (OpenAI-compatible)
- **Base URL Padrao**: `http://127.0.0.1:11434`
- **Pass-through**: Payloads de request e streams de response sao encaminhados diretamente sem reidratacao ou modificacao de JSON.

---

## 8. Headers

Headers permitidos para upstream devem ser allowlisted.

Encaminhar:

```text
Content-Type
Accept
Authorization transformado pela config
User-Agent do gateway
Request-ID gerado
```

Nao encaminhar sem revisao:

```text
cookies
headers internos
chaves de outro provedor
headers de tenant sensiveis
```

---

## 9. Erros

Antes de iniciar stream:

```text
responder status HTTP adequado
```

Depois de iniciar stream:

```text
encerrar stream preservando semantica possivel
registrar erro terminal em telemetria
cancelar upstream se cliente desconectar
```

---

## 10. Testes de Contrato

Cada provedor deve ter fixtures:

```text
input request
expected upstream request
upstream stream
expected client stream
expected telemetry
```

Fixtures devem ficar em:

```text
tests/fixtures/<provider>/
```
