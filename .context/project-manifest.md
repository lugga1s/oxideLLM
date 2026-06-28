# Manifesto de Contexto Mestre: oxideLLM

Status: contexto estrategico do projeto  
Finalidade: orientar decisoes tecnicas, posicionamento publico, narrativa de produto e prioridades de implementacao

---

## 1. Identidade do Projeto

O projeto, codinome **oxideLLM**, e um gateway/proxy reverso unificado de LLMs construido para resolver gargalos severos de performance em ambientes corporativos de IA.

A tese central e simples: gateways tradicionais de IA, quando implementados sobre runtimes interpretados e acoplados a logging/tracing/persistencia sincronos, degradam drasticamente a vazao sob concorrencia alta. O projeto existe para substituir esse padrao por uma fundacao compilada, assincrona, orientada a baixo overhead, streaming eficiente e telemetria desacoplada.

O produto deve ser:

- gratuito para uso local;
- simples de instalar e configurar;
- orientado por benchmarks empiricos;
- tecnicamente rigoroso;
- aberto o suficiente para atrair contribuidores;
- poderoso o bastante para justificar patrocinio corporativo voluntario.

---

## 2. Perfil do Autor e Filosofia

O autor e lider do projeto atua como engenheiro de nivel enterprise-grade, com foco em:

- resiliencia de sistemas;
- arquitetura distribuida;
- escalabilidade profunda;
- economia de custos de infraestrutura;
- engenharia de performance;
- confiabilidade operacional.

A filosofia do projeto e entregar valor tecnico extremo sem friccao comercial inicial. O desenvolvedor moderno evita ferramentas com paywalls prematuros, configuracao pesada e dependencia de SaaS fechado. Por isso, o projeto deve oferecer uma experiencia local rapida, util e mensuravelmente superior.

A ambicao publica e gerar crescimento organico forte no GitHub por meio de utilidade real, narrativa clara e provas empiricas de performance.

---

## 3. Contexto de Mercado

O projeto se posiciona dentro do ecossistema COSS, Commercial Open Source Software, e de estrategias open-core leves.

Referencias de mercado e inspiracao:

- **Firecrawl e Crawl4AI**: ferramentas voltadas a ingestao e tratamento de dados limpos para LLMs.
- **Browser Use**: mecanismos de agentes autonomos baseados em navegacao virtual.
- **Mem0**: memoria persistente e otimizacao de contexto para agentes.
- **Langfuse e LiteLLM**: LLMOps, gateways de API, observabilidade e infraestrutura de IA.

Essas referencias demonstram que projetos abertos, tecnicamente fortes e imediatamente uteis conseguem atrair comunidade, estrelas, patrocinadores e empresas usuarias.

---

## 4. Problema Critico

O problema que o projeto resolve e o gargalo de performance na camada de gateways/proxies de IA.

Em ambientes de producao corporativa, ferramentas tradicionais podem acoplar as seguintes responsabilidades no mesmo caminho critico:

- proxy de requisicao;
- traducao de payloads;
- tracing;
- logging estruturado;
- contagem de tokens;
- persistencia de metadados;
- escrita em bancos relacionais;
- integracao com Redis ou filas auxiliares.

Esse acoplamento transforma o gateway em gargalo serializante. Sob concorrencia alta, principalmente em cenarios com 500 requisicoes simultaneas, a eficiencia pode cair drasticamente. A pesquisa interna do projeto documentou degradacao de ate **75,6%** em relacao a uma conexao direta ao motor de inferencia.

O diagnostico arquitetural e que operacoes imediatas de rede precisam ser desacopladas da persistencia analitica.

---

## 5. Solucao Tecnica

A solucao e um gateway de alta performance escrito em linguagem compilada de baixo nivel, inicialmente avaliada entre **Go** e **Rust**.

O sistema deve sustentar:

- throughput massivo;
- latencia P99 estavel sob estresse;
- streaming SSE eficiente;
- traducao entre provedores;
- alocacao minima no caminho quente;
- telemetria fora do caminho critico;
- backpressure explicito;
- micro-batching para persistencia.

### 5.1 Camada de Proxy Zero-Copy

O gateway deve traduzir esquemas de provedores de IA para uma interface unificada, preferencialmente compativel com OpenAI, evitando parse completo e reserializacao quando o payload puder ser encaminhado em modo pass-through.

O processamento de SSE deve ocorrer como stream de bytes, com parse incremental apenas para extracao de metadados e medicao de tokens/latencia.

### 5.2 Buffer Assincrono de Telemetria

Logs e metricas devem ser coletados por um mecanismo assincrono de baixa contencao, idealmente usando ring buffer ou filas bounded por shard.

A meta conceitual e que a publicacao de metadados custe microssegundos ou menos em condicoes normais, sem bloquear a thread/task responsavel por responder ao cliente.

### 5.3 Engine de Ingestao Desacoplada

Um processo ou conjunto de workers em background deve consumir os eventos de telemetria e persistir em lote.

Destinos possiveis:

- ClickHouse;
- Parquet;
- Kafka/Redpanda/NATS;
- OpenTelemetry collector;
- Postgres apenas para estado operacional pequeno, nunca como write path sincrono de cada request.

---

## 6. Estrategia de Monetizacao Light

O projeto deve funcionar como uma ferramenta 100% gratuita para uso local, com monetizacao baseada em doacoes e patrocinio voluntario.

O gatilho economico e reducao de TCO. Se o gateway economiza CPU, memoria, licencas e risco operacional, empresas podem considerar racional apoiar financeiramente o mantenedor.

Mecanismos previstos:

- GitHub Sponsors;
- Open Collective;
- Buy Me a Coffee;
- patrocinio corporativo mensal;
- mural de patrocinadores no README;
- mensagens discretas no terminal convidando apoio voluntario.

A monetizacao nao deve enfraquecer a promessa central: ferramenta poderosa, util, local e sem barreiras pesadas.

---

## 7. Diretriz de Execucao Publica

O repositorio deve ser orientado por benchmarks reais, colocados em destaque desde o inicio.

O README deve mostrar:

- problema concreto;
- benchmark direto contra baseline;
- instalacao em menos de 3 minutos;
- exemplo minimo funcional;
- comparacao clara com gateways tradicionais;
- arquitetura resumida;
- status de suporte a provedores;
- link para patrocinio;
- roadmap tecnico.

A narrativa publica deve evitar promessas vagas. O projeto deve vencer por demonstracao empirica.

---

## 8. Norte Tecnico

As decisoes tecnicas devem obedecer aos seguintes principios:

- mover bytes rapidamente;
- interpretar apenas o necessario;
- transformar apenas quando inevitavel;
- nunca bloquear cliente por telemetria analitica;
- usar memoria com disciplina;
- preferir conexoes persistentes e multiplexadas;
- aplicar backpressure cedo;
- medir tudo;
- publicar resultados reprodutiveis;
- manter instalacao e operacao simples.

---

## 9. Relacao com Outros Documentos de Contexto

Este manifesto define o norte estrategico do projeto.

O documento `.context/bottlenecks.md` define a especificacao tecnica detalhada da camada proxy base e da arquitetura para eliminar a degradacao de 75,6%.

Juntos, estes documentos formam a base de contexto para decisoes futuras de produto, arquitetura, implementacao e comunicacao publica.
