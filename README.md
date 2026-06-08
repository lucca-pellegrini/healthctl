# healthctl

**Ferramenta CLI para rastreamento de saúde pessoal** - Contribuindo para o ODS 3 (Saúde e Bem-Estar)

## Sobre o Projeto

O `healthctl` é uma solução de software desenvolvida como trabalho prático da disciplina de Engenharia de Software, abordando o **Objetivo de Desenvolvimento Sustentável 3 (ODS 3) - Saúde e Bem-Estar**.

A aplicação permite que usuários registrem e acompanhem métricas de saúde pessoal através de uma interface de linha de comando (CLI) eficiente e um dashboard visual, promovendo hábitos saudáveis e autocuidado.

## Tecnologias

- **Linguagem:** Rust (Edition 2024)
- **CLI:** Clap
- **Persistência:** SQLite (via SQLx)
- **IPC:** Unix Domain Sockets
- **Dashboard:** Tauri
- **Licença:** GPL-2.0-or-later

## Instalação

```bash
# Clonar o repositório
git clone https://github.com/lucca-pellegrini/healthctl.git
cd healthctl

# Compilar
cargo build --release

# Executar
./target/release/healthctl --help
```

## Uso Básico

```bash
# Registrar uma corrida
healthctl add activity run --duration 30m --distance 5km --calories 300

# Registrar hidratação
healthctl add hydration 500ml

# Registrar sono
healthctl add sleep --start "yesterday 23:00" --end "today 07:00"

# Ver status do dia
healthctl status

# Listar eventos da semana
healthctl list --week

# Gerar relatório
healthctl report week
```

## Estrutura do Projeto

```
healthctl/
├── crates/
│   ├── healthctl/          # CLI principal
│   ├── healthctl-daemon/   # Daemon com SQLite
│   ├── healthctl-lib/      # Biblioteca compartilhada
│   └── healthctl-dashboard/# Dashboard Tauri
├── docs/                   # Documentação do projeto
│   ├── TP1.md             # Requisitos e casos de uso
│   ├── arquitetura.md     # Arquitetura C4
│   └── testes.md          # Plano de testes
└── Videos/                 # Vídeos de demonstração
```

## Documentação

A documentação completa do projeto está disponível na pasta [`docs/`](./docs/):

- [TP1 - Definição do Problema e Requisitos](./docs/TP1.md)
- [Arquitetura do Sistema (C4 Model)](./docs/arquitetura.md)
- [Plano de Testes](./docs/testes.md)

## Licença

Este projeto está licenciado sob a GPL-2.0-or-later. Veja o arquivo [LICENSE](./LICENSE) para mais detalhes.

## Autor

Desenvolvido como trabalho prático de Engenharia de Software.
