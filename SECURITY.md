# Security Policy

## Dados Sensiveis

O gateway pode lidar com prompts, completions e API keys. Por padrao:

- nao logar prompts completos;
- nao logar completions completas;
- nao logar `Authorization`;
- nao persistir headers sensiveis;
- nao commitar `.env`.

## Reportar Vulnerabilidade

Enquanto o projeto nao tiver canal publico definitivo, reporte vulnerabilidades criando uma issue privada ou contatando o mantenedor por canal definido no GitHub do projeto.

## Principios

- menor privilegio;
- allowlist de headers upstream;
- opt-in explicito para auditoria de payload;
- redacao/mascara de dados sensiveis;
- segredos via ambiente ou secret manager, nunca no repo.

