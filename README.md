# Carteira de Investimentos

> Desafio final do curso **Rust Fullstack** (DIO / Santander). Projeto
> desenvolvido a partir do [repositório base](https://github.com/digitalinnovationone/rust-fullstack-carteira-investimentos)
> fornecido pelo curso.

## O que faz o projeto

Uma aplicação fullstack em Rust para cadastrar e acompanhar ativos de
investimento (nome + valor unitário). Tem cadastro/login de usuário
(com criação automática de conta no primeiro login), sessão via cookie
+ JWT, um dashboard mostrando os ativos disponíveis e o valor total da
carteira, e uma API JSON para gerenciar os ativos.

## Tecnologias usadas

- **Axum** — framework web / rotas
- **SQLx + PostgreSQL** — persistência
- **Askama** — templates HTML (dashboard e login)
- **jsonwebtoken** + **axum-extra** (cookies) — autenticação de sessão
- **password-auth** — hash de senha
- **Docker Compose** — banco de dados local
- **insta** — testes de snapshot

## Como executar a aplicação

1. Subir o banco:
   ```
   docker compose up -d
   ```
2. Instalar o `sqlx-cli` (uma vez só) e rodar as migrations:
   ```
   cargo install sqlx-cli --no-default-features --features postgres
   sqlx migrate run
   ```
3. Rodar o servidor:
   ```
   cargo run
   ```
4. Acessar `http://localhost:3000` no navegador. No primeiro acesso, o
   formulário de login já cadastra a conta automaticamente.

O `.env` já vem com valores padrão de desenvolvimento. `JWT_SECRET` e
`ADMIN_API_KEY` devem ser trocados antes de qualquer uso fora da sua
própria máquina.

## Melhorias implementadas em cima do projeto base

O projeto base entrega a API de ativos, autenticação e a estrutura do
servidor, mas não tinha nenhuma página além do login (a rota `/` só
devolvia um texto simples) e não validava dados de entrada. A partir
dele, implementei:

- **Dashboard em Askama** (`templates/dashboard.html`): lista os ativos
  cadastrados com preço em formato brasileiro (R$), em vez do texto
  puro que existia antes.
- **Valor total da carteira**: soma do `unit_value` de todos os ativos,
  calculada em `index()` (`src/routes/frontend.rs`) e exibida em
  destaque no topo do dashboard.
- **Logout** (`POST /logout`): não existia nenhuma forma de encerrar a
  sessão antes.
- **Validações de entrada**: nome de ativo vazio, valor negativo,
  usuário vazio e senha curta agora retornam erro `400` com mensagem
  clara, em vez de ir direto pro banco sem checagem.
- **Configuração via ambiente**: a chave de JWT e a chave de admin
  eram constantes fixas no código-fonte; passaram para variáveis de
  ambiente (`src/config.rs`).
- **Endpoint `GET /api/me`**: retorna o usuário autenticado a partir do
  cookie/JWT.
- **Reorganização do `repository.rs`** em `repository/{assets,users}.rs`,
  separando responsabilidades por assunto.

## Como testar

```
cargo test
```

Cobre: a lógica de validação da API de ativos (nome vazio, valor
negativo, nome duplicado, ativo inexistente), criação/listagem/edição
de ativos (com snapshots do `insta`), e a formatação do valor total em
BRL. Alguns testes usam `#[sqlx::test]` e precisam do Postgres do
`docker compose` rodando.

Pra testar manualmente pelo navegador: crie uma conta pelo formulário
de login e veja o dashboard. Pra cadastrar um ativo (precisa da
`ADMIN_API_KEY` do `.env`):
```
curl -X POST http://localhost:3000/api/assets \
  -H "Authorization: dev-admin-key-troque-em-producao" \
  -H "Content-Type: application/json" \
  -d '{"name": "Bitcoin", "unit_value": 10.0}'
```

## O que eu aprendi durante o desafio

Durante o desenvolvimento desse projeto e a implementação de melhoras, pude aprender muito sobre o ecossistema web do Rust. Alguns dos conceitos os quais consolidei foram:

1. Descobri que o Axum resolve dependências de forma muito elegante diretamente na assinatura das funções. Quando coloco um argumento como o Repository ou uma struct Admin, o framework utiliza a trait FromRequestParts. Na prática, antes mesmo de a requisição executar a lógica da função, o Axum inspeciona o estado da aplicação e os cabeçalhos, extrai o que é necessário e injeta o objeto pronto para uso.

2. Acostumado a modelar bancos e escrever queries diretamente no PostgreSQL, a macro "query_as!" do SQLx mudou muito minha perspectiva. Em vez de enviar strings de texto que só dariam erro em tempo de execução, o SQLx se conecta ao banco de dados durante a compilação. Ele verifica se a sintaxe está correta, se as tabelas/colunas existem e se os tipos do banco correspondem perfeitamente às structs do Rust. Se a query estiver errada, o código simplesmente não compila.

3. Um erro clássico que me confundiu bastante no início foi quando o compilador reclamava que the trait bound is not satisfied para a trait Handler. Com o tempo, aprendi que apenas significa que um dos parâmetros da minha rota não era um extractor válido.

4. Ideias de melhorias:

- Implementar paginação na listagem de ativos da API e no Dashboard.

- Criar o Dockerfile da aplicação Rust e subir o ecossistema inteiro de forma isolada em contêineres, facilitando o deploy.

- Refinar os templates do Askama integrando um framework CSS como Tailwind para um visual mais profissional.
