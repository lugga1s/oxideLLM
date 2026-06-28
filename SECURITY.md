# Security Policy

oxideLLM e um gateway de IA e pode processar prompts, completions, headers de
autenticacao e metadados de uso. A regra padrao e simples: dados sensiveis nao
devem entrar em logs, exemplos, issues publicas ou commits.

## Versoes Suportadas

Enquanto o projeto estiver antes do `1.0.0`, a politica de suporte acompanha a
branch `main` e a versao mais recente publicada. Correcoes criticas podem ser
backportadas quando houver tag estavel afetada.

| Versao | Suporte |
|---|---|
| `main` | sim |
| release mais recente | sim |
| releases antigas pre-1.0 | melhor esforco |

## Reportar Vulnerabilidade

Nao abra issue publica com detalhes exploraveis.

Reporte vulnerabilidades de forma responsavel por um destes canais:

- use GitHub Security Advisories, se habilitado no repositorio;
- envie uma mensagem privada ao mantenedor pelo perfil GitHub do projeto;
- se nenhum canal privado estiver disponivel, abra uma issue publica minima
  pedindo um canal seguro, sem incluir payloads, tokens, PoC completa ou dados
  de terceiros.

Inclua, quando possivel:

- versao ou commit afetado;
- impacto esperado;
- passos minimos de reproducao;
- configuracao relevante sem segredos;
- se a falha envolve vazamento, execucao remota, bypass de auth ou DoS.

## Tempo de Resposta

Metas iniciais de resposta:

- confirmacao de recebimento: ate 7 dias;
- triagem inicial: ate 14 dias;
- plano de correcao para falha critica: melhor esforco, com prioridade maxima.

Esses prazos podem mudar conforme o projeto amadurecer e tiver canal de
seguranca formal.

## Dados Sensiveis

Por padrao:

- nao logar prompts completos;
- nao logar completions completas;
- nao logar `Authorization`;
- nao persistir headers sensiveis;
- nao commitar `.env`;
- nao colocar API keys em exemplos;
- mascarar tokens em logs e reports.

## Principios de Seguranca

- menor privilegio;
- allowlist de headers upstream;
- opt-in explicito para auditoria de payload;
- redacao ou mascara de dados sensiveis;
- segredos via ambiente ou secret manager, nunca no repo;
- telemetria bounded e assincrona para evitar DoS por fila infinita;
- falhas de persistencia nao devem expor prompts nem bloquear o caminho critico.

## Escopo Fora da Politica

Estes casos normalmente nao sao tratados como vulnerabilidades criticas:

- benchmarks de performance sem exploracao de seguranca;
- problemas em dependencias ja corrigidos por update simples;
- falhas que exigem acesso local irrestrito ao host;
- reports sem reproducao minima e sem impacto demonstravel.
