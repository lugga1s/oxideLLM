# Estrategia de Licenciamento

Status: decisao inicial, revisar antes do primeiro release publico

---

## 1. Recomendacao Atual

Usar **AGPL-3.0-or-later**.

Motivo:

- o projeto e um gateway de rede;
- o valor principal pode ser explorado como SaaS;
- queremos evitar forks comerciais fechados que melhoram o gateway e nao devolvem nada;
- queremos preservar margem para versoes futuras da AGPL;
- o projeto busca comunidade e patrocinio, nao venda imediata de licenca permissiva.

Identificador SPDX:

```text
AGPL-3.0-or-later
```

---

## 2. O Que a AGPL Protege

A AGPL e uma licenca copyleft forte desenhada para software usado via rede.

Na pratica:

```text
Se alguem modificar o gateway e oferecer esse software modificado como servico de rede,
essa pessoa/empresa deve disponibilizar o codigo-fonte correspondente das modificacoes
aos usuarios que interagem com o servico.
```

Isso e importante para um gateway de IA porque o concorrente natural nao e apenas alguem que distribui binario. E tambem alguem que roda o gateway como servico hospedado.

---

## 3. Trade-offs

Vantagens:

- protege contra apropriacao SaaS fechada;
- reforca reciprocidade da comunidade;
- combina com infraestrutura de rede;
- incentiva contribuicoes upstream;
- sustenta narrativa de COSS forte.

Custos:

- algumas empresas evitam AGPL por politica interna;
- pode reduzir adocao em ambientes corporativos conservadores;
- integracoes com codigo proprietario precisam de mais cuidado;
- pode exigir esclarecimento juridico para patrocinadores.

---

## 4. Alternativas Consideradas

### Apache-2.0

Melhor para adocao ampla, pior para protecao contra uso fechado.

Boa se o objetivo principal virar:

```text
maxima distribuicao, minimo atrito corporativo, monetizacao por servicos/suporte.
```

### MIT

Extremamente permissiva e simples, mas fraca para proteger arquitetura.

Nao recomendada para este projeto se a prioridade for evitar fork SaaS fechado.

### Dual License

Possivel no futuro:

```text
AGPL-3.0-or-later para comunidade
licenca comercial para empresas que precisam integrar de forma proprietaria
```

Esse modelo exige governanca de contribuicoes, CLA/DCO e clareza juridica.

### Open Core

Possivel no futuro, mas nao recomendado no inicio. O manifesto do projeto prioriza valor gratuito local e crescimento organico.

---

## 5. Decisao Para o Release 0.x

Manter:

```text
AGPL-3.0-or-later
```

Reavaliar antes do `1.0.0` se:

- empresas-alvo rejeitarem AGPL em massa;
- o projeto buscar integracao proprietaria embarcada;
- surgir plano real de dual licensing;
- houver patrocinador corporativo exigindo alternativa.

---

## 6. Checklist de Compliance

- `LICENSE` contem texto completo da GNU AGPL v3.
- `Cargo.toml` usa `AGPL-3.0-or-later`.
- arquivos Rust usam `SPDX-License-Identifier: AGPL-3.0-or-later`.
- `README.md` explica a licenca.
- `CONTRIBUTING.md` informa que contribuicoes seguem a mesma licenca.
- mudanca de licenca exige ADR.

---

## 7. Nota Importante

Este documento e orientacao tecnica e estrategica, nao aconselhamento juridico. Antes de release comercial relevante, validar a estrategia com advogado especializado em open source.
