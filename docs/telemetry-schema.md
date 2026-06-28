# Schema de Telemetria

Status: contrato inicial de eventos

---

## 1. Principio

Telemetria e dado operacional, nao payload de usuario.

Por padrao, eventos nao devem conter:

- prompt completo;
- completion completa;
- API keys;
- headers sensiveis;
- corpo bruto;
- PII.

---

## 2. Evento Final Canonico

```json
{
  "event_type": "request_completed",
  "schema_version": 1,
  "request_id": "uuid",
  "tenant_id": "string",
  "route_id": "string",
  "provider_id": "string",
  "model_requested": "string",
  "model_resolved": "string",
  "started_at_ms": 0,
  "completed_at_ms": 0,
  "ttft_ms": 0,
  "total_latency_ms": 0,
  "bytes_in": 0,
  "bytes_out": 0,
  "input_tokens": 0,
  "output_tokens": 0,
  "token_source": "provider",
  "status": "ok",
  "error_class": null
}
```

---

## 3. Eventos Internos

### RequestStarted

Emitido no ingresso.

Campos:

```text
request_id
tenant_id
started_at_ms
route_hint
```

### UpstreamSelected

Emitido apos roteamento, se necessario.

Campos:

```text
request_id
route_id
provider_id
model_resolved
```

### FirstTokenObserved

Emitido no primeiro delta util.

Campos:

```text
request_id
ttft_ms
provider_id
model_resolved
```

### RequestCompleted

Emitido no fim normal.

Campos:

```text
request_id
completed_at_ms
total_latency_ms
bytes_in
bytes_out
tokens
status
```

### RequestFailed

Emitido em erro.

Campos:

```text
request_id
failed_at_ms
error_class
upstream_status
partial_stream
```

### ClientDisconnected

Emitido quando cliente fecha conexao antes do fim.

Campos:

```text
request_id
disconnected_at_ms
bytes_out
partial_tokens
```

---

## 4. Politica de Fila Cheia

```text
debug events: descartar
first token events: descartar se necessario, contador obrigatorio
final events: tentar preservar
billing strict: futuro modo separado
```

Todo drop deve incrementar contador.

---

## 5. Batch

Batch inicial:

```json
{
  "schema_version": 1,
  "batch_id": "uuid",
  "created_at_ms": 0,
  "event_count": 1000,
  "events": []
}
```

Flush:

```text
max_age_ms = 500
max_events = 1000
```

---

## 6. Idempotencia

Chave recomendada:

```text
request_id + event_type + attempt_id
```

Sinks devem tolerar duplicata.

---

## 7. Retencao

Padrao local:

```text
metricas agregadas: longa
eventos finais: configuravel
payload bruto: desabilitado por padrao
```

Auditoria de payload requer opt-in explicito.
