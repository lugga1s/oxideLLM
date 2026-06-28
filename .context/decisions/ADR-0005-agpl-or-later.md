# ADR-0005: AGPL-3.0-or-later

Status: aceito  
Data: 2026-06-28

## Contexto

O projeto e um gateway de IA executado como software de rede. O risco de apropriacao mais relevante e um terceiro modificar o gateway, hospedar como SaaS e nao devolver melhorias para a comunidade.

## Decisao

Licenciar o projeto como:

```text
AGPL-3.0-or-later
```

## Racional

AGPL e adequada para software de rede porque cobre o caso de uso via servico remoto. A opcao `or-later` preserva flexibilidade para versoes futuras da licenca publicadas pela Free Software Foundation.

## Consequencias Positivas

- maior protecao contra forks SaaS fechados;
- reciprocidade mais forte;
- alinhamento com infraestrutura de rede;
- narrativa COSS mais defensavel.

## Consequencias Negativas

- pode reduzir adocao corporativa em empresas avessas a AGPL;
- pode exigir licenca comercial futura para clientes especificos;
- contribuicoes precisam manter clareza de direitos.

## Reavaliacao

Reavaliar antes do `1.0.0` se:

- AGPL bloquear parcerias essenciais;
- houver plano de dual licensing;
- contribuidores corporativos pedirem CLA/DCO;
- estrategia de monetizacao mudar.
