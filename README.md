# healthctl

**Ferramenta CLI para rastreamento de saúde pessoal** - Contribuindo para o ODS
3 (Saúde e Bem-Estar)

## Sobre o Projeto

O `healthctl` é uma solução de software desenvolvida como trabalho prático da
disciplina de Engenharia de Software, abordando o **Objetivo de Desenvolvimento
Sustentável 3 (ODS 3) - Saúde e Bem-Estar**.

A aplicação permite que usuários registrem e acompanhem métricas de saúde
pessoal através de uma interface de linha de comando (CLI) eficiente e um
dashboard visual, promovendo hábitos saudáveis e autocuidado.

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

cargo build --release --all # Compilar
./target/release/healthctl --help # Executar
cp ./target/release/healthctl{,-daemon,-dashboard} ~/.local/bin/ # Instalar em .local/bin
```

Também pode-se compilar uma [AppImage](https://appimage.org/) com todos os
componentes em um único executável (é necessário ter
[appimagetool](https://github.com/AppImage/appimagetool) no `PATH`):

```bash
./utils/build_appimage.sh # Resultado em target/appimage/healthctl-<ARCH>.AppImage
cp target/appimage/healthctl-x86_64.AppImage ~/.local/bin/healthctl
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

## Completação no Shell (zsh)

O `healthctl` inclui uma função de completação para **zsh** em
[`completions/_healthctl`](./completions/_healthctl). Além de completar
subcomandos e flags, ela consulta o daemon em tempo real para sugerir:

- **IDs de eventos** (em `show`, `edit`, `remove`/`rm`, `clone`), em ordem
  cronológica (mais recentes primeiro) e anotados com o tipo, a data e as tags
  do evento — assim como `git show <TAB>` anota commits.
- **Tags** (em `add --tag`, `list --tag`, `clone --tag`), ordenadas da mais
  recente para a mais antiga (limitadas às ~30 mais recentes).

Para instalar (exemplo):

- Copie a função para um diretório no seu $fpath
  ```bash
  mkdir -p ~/.config/zsh/zfunc
  cp completions/_healthctl ~/.config/zsh/zfunc/
  ```
- Em ~/.zshrc, *antes* de `compinit`:
  ```bash
  fpath+="${HOME}/.config/zsh/zfunc"
  autoload -Uz compinit
  compinit
  ```

> [!NOTE]
> As sugestões dinâmicas (IDs e tags) aparecem apenas quando o daemon já está
> em execução; caso contrário, a completação simplesmente não oferece esses
> candidatos (sem erros e sem iniciar o daemon).

## Estrutura do Projeto

```
healthctl/
├── crates/
│   ├── healthctl/          # CLI principal
│   ├── healthctl-daemon/   # Daemon com SQLite
│   ├── healthctl-lib/      # Biblioteca compartilhada
│   └── healthctl-dashboard/# Dashboard Tauri
├── completions/            # Completação de shell (zsh)
├── docs/                   # Documentação do projeto
│   ├── requisitos.md       # Requisitos e casos de uso
│   ├── arquitetura.md      # Arquitetura C4
│   └── testes.md           # Plano de testes
└── Videos/                 # Vídeos de demonstração
```

## Documentação

A documentação completa do projeto está disponível na pasta [`docs/`](./docs/):

- [Definição do Problema e Requisitos](./docs/requisitos.md)
- [Arquitetura do Sistema (C4 Model)](./docs/arquitetura.md)
- [Plano de Testes](./docs/testes.md)

## Licença

Este projeto está licenciado sob a GPL-2.0-or-later. Veja o arquivo [LICENSE](./LICENSE) para mais detalhes.

## Autor

Desenvolvido como trabalho prático de Engenharia de Software.
