# ADR-0007: WSL2 como ambiente de benchmark e profiling

Status: aceito  
Data: 2026-06-28  
Autor: sessao de agente autonomo

## Contexto

O projeto esta sendo desenvolvido em Windows. Testes funcionais, compilacao e validacao de codigo funcionam perfeitamente. Porem, dois problemas foram identificados:

1. Benchmarks de alta concorrencia (1000+ VUs) sofrem com limite de portas TCP do Windows (dynamic port range e TIME_WAIT), gerando taxas de erro de ate 50% que nao refletem o desempenho real do gateway.

2. Ferramentas de profiling essenciais (perf, heaptrack, flamegraph) sao exclusivas de Linux e nao possuem equivalentes adequados em Windows.

Os resultados do Stage 2 com 10 VUs mostraram overhead de apenas 0,03ms, mas o teste com carga alta foi inconclusivo por limitacoes do ambiente, nao do codigo.

## Decisao

Usar WSL2 (Windows Subsystem for Linux) como ambiente obrigatorio para benchmarks de alta concorrencia e profiling. Windows continua como ambiente primario de desenvolvimento.

Divisao de responsabilidades:

- Windows: desenvolvimento, cargo check/test/clippy/fmt, testes funcionais, Ollama
- WSL2: benchmarks k6 com 100+ VUs, perf stat, heaptrack, flamegraph, DHAT

## Consequencias

- Benchmarks publicaveis serao reproduziveis em Linux
- Sem necessidade de maquina extra, VPS ou dual boot
- WSL2 acessa os mesmos arquivos do projeto em /mnt/c/
- Resultados de benchmark devem registrar o ambiente (Windows vs WSL2)
- Agents devem verificar o ambiente antes de rodar benchmarks de alta concorrencia

## Alternativas consideradas

1. Manter tudo em Windows: descartado porque gera resultados de benchmark nao confiaveis e impossibilita profiling
2. VPS ou maquina Linux dedicada: funcional mas desnecessario quando WSL2 resolve
3. Docker com Linux: possivel mas adiciona overhead de rede e nao resolve profiling de host
