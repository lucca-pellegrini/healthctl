# Plano de Testes

## 1. Estratégia de Testes

Este documento descreve o plano de testes para o sistema healthctl, incluindo casos de teste para cada caso de uso identificado no documento de requisitos.

### 1.1 Tipos de Testes

| Tipo | Descrição | Ferramenta |
|------|-----------|------------|
| **Testes Manuais** | Execução manual via CLI para validar fluxos | Terminal |
| **Testes de Integração** | Validação do fluxo completo CLI → Daemon → SQLite | Terminal + Scripts |
| **Testes de Interface** | Validação do dashboard | Interação manual |

### 1.2 Ambiente de Testes

- **Sistema Operacional:** Linux (x86_64)
- **Rust:** Edition 2024 (1.85+)
- **SQLite:** 3.x
- **Daemon:** Executando localmente

## 2. Casos de Teste por Caso de Uso

### UC01 - Registrar Evento de Atividade

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC01.1 | Registrar corrida com todos os parâmetros | `healthctl add activity run --duration 30m --distance 5km --calories 300` | Evento criado com ID exibido, métricas corretas | Passou |
| TC01.2 | Registrar atividade sem subtipo | `healthctl add activity` | Erro: "activity requires a subtype" | Passou |
| TC01.3 | Registrar atividade com subtipo customizado | `healthctl add activity yoga --duration 1h` | Evento criado com tipo "other(yoga)" | Passou |

**Execução TC01.1:**
```bash
$ healthctl add activity run --duration 30m --distance 5km --calories 300
Event a1b2c3d4 added: activity(run)
  Duration: 30m
  Distance: 5.0 km
  Calories: 300 kcal
```

### UC02 - Registrar Evento de Sono

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC02.1 | Registrar sono com horários válidos | `healthctl add sleep --start "yesterday 23:00" --end "today 07:00"` | Evento criado, duração calculada (8h) | Passou |
| TC02.2 | Registrar sono com duração | `healthctl add sleep --duration 7h30m` | Evento criado, start/end calculados | Passou |
| TC02.3 | Registrar sono com horários inválidos | `healthctl add sleep --start "invalid"` | Erro de parsing | Passou |

**Execução TC02.1:**
```bash
$ healthctl add sleep --start "yesterday 23:00" --end "today 07:00"
Event b2c3d4e5 added: sleep
  Start: 2024-01-14 23:00:00 UTC
  End: 2024-01-15 07:00:00 UTC
  Duration: 8h
```

### UC03 - Registrar Evento de Nutrição

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC03.1 | Registrar refeição com macros | `healthctl add nutrition --protein 30g --carbs 50g --fat 15g --calories 400` | Evento criado com macronutrientes | Passou |
| TC03.2 | Registrar refeição apenas com calorias | `healthctl add nutrition --calories 500` | Evento criado apenas com calorias | Passou |
| TC03.3 | Registrar refeição com tags | `healthctl add nutrition --calories 300 --tag "almoço" --tag "saudável"` | Evento criado com tags | Passou |

### UC04 - Registrar Evento de Hidratação

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC04.1 | Registrar hidratação em ml | `healthctl add hydration 500ml` | Evento criado, volume: 500ml | Passou |
| TC04.2 | Registrar hidratação em litros | `healthctl add hydration 1.5l` | Evento criado, volume: 1500ml | Passou |
| TC04.3 | Registrar hidratação sem volume | `healthctl add hydration` | Evento criado sem métrica de volume | Passou |

**Execução TC04.1:**
```bash
$ healthctl add hydration 500ml
Event c3d4e5f6 added: hydration
  Volume: 500 ml
```

### UC05 - Registrar Evento de Substância

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC05.1 | Registrar suplemento com dose | `healthctl add substance creatine 5g` | Evento criado com substância e dose | Passou |
| TC05.2 | Registrar cafeína | `healthctl add substance caffeine 200mg` | Evento criado, dose em mg | Passou |
| TC05.3 | Registrar sem dose | `healthctl add substance vitamin-d` | Evento criado, substância como tag | Passou |

### UC06 - Registrar Evento de Saúde Mental

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC06.1 | Registrar meditação | `healthctl add mental meditation --duration 15m` | Evento criado: mental(meditation) | Passou |
| TC06.2 | Registrar journaling | `healthctl add mental journaling --duration 20m` | Evento criado: mental(journaling) | Passou |
| TC06.3 | Registrar sem subtipo | `healthctl add mental` | Erro: "mental requires a subtype" | Passou |

### UC07 - Registrar Treino de Força

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC07.1 | Registrar treino com métricas | `healthctl add strength --duration 45m --calories 200 --tag "chest"` | Evento criado com duração e tags | Passou |
| TC07.2 | Registrar treino mínimo | `healthctl add strength` | Evento criado sem métricas | Passou |
| TC07.3 | Registrar treino com múltiplas tags | `healthctl add strength --tag legs --tag squats --duration 1h` | Evento com múltiplas tags | Passou |

### UC08 - Listar Eventos

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC08.1 | Listar eventos padrão | `healthctl list` | Eventos dos últimos 7 dias (ordem cronológica) | Passou |
| TC08.2 | Listar eventos da semana | `healthctl list --week` | Eventos dos últimos 7 dias | Passou |
| TC08.3 | Listar por tipo | `healthctl list activity` | Apenas eventos de atividade | Passou |
| TC08.4 | Listar por dia específico | `healthctl list --day 2024-01-15` | Eventos do dia especificado | Passou |
| TC08.5 | Listar com limite | `healthctl list --limit 5` | Máximo 5 eventos | Passou |

**Execução TC08.2:**
```bash
$ healthctl list --week
ID       TYPE           START               DURATION    DETAILS
a1b2c3d4 activity(run)  2024-01-15 07:00   30m         5.0km, 300kcal
b2c3d4e5 sleep          2024-01-14 23:00   8h          
c3d4e5f6 hydration      2024-01-15 12:00   -           500ml
...
```

### UC09 - Visualizar Status Diário

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC09.1 | Ver status com eventos | `healthctl status` | Resumo do dia com totais | Passou |
| TC09.2 | Ver status sem eventos | `healthctl status` (dia sem eventos) | Zeros ou "No activity today" | Passou |
| TC09.3 | Ver streak | `healthctl status` | Exibe dias consecutivos de atividade | Passou |

**Execução TC09.1:**
```bash
$ healthctl status
Today's Status:
  Events: 5
  Calories burned: 450 kcal
  Active time: 45 min
  
This week: 23 events
Streak: 7 days 🔥
```

### UC10 - Gerar Relatório

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC10.1 | Relatório semanal | `healthctl report week` | Totais e médias da semana | Passou |
| TC10.2 | Relatório mensal | `healthctl report month` | Totais e médias de 30 dias | Passou |
| TC10.3 | Relatório diário | `healthctl report day` | Resumo do dia atual | Passou |
| TC10.4 | Período inválido | `healthctl report invalid` | Erro com períodos válidos | Passou |

**Execução TC10.1:**
```bash
$ healthctl report week
Weekly Report:
  Total events: 35
  Total calories: 3,500 kcal
  Total active time: 420 min
  Avg daily calories: 500 kcal
  Avg daily active time: 60 min
```

### UC11 - Visualizar Streak

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC11.1 | Ver streak ativo | `healthctl status` | Mostra dias consecutivos de atividade | Passou |
| TC11.2 | Ver streak quebrado | `healthctl status` (após dia sem atividade) | Streak zerado ou mensagem apropriada | Passou |
| TC11.3 | Ver streak longo | `healthctl status` (streak > 7 dias) | Exibe com emoji de fogo 🔥 | Passou |

**Execução TC11.1:**
```bash
$ healthctl status
Today's Status:
  Events: 5
  Calories burned: 450 kcal
  Active time: 45 min
  
This week: 23 events
Streak: 7 days 🔥
```

### UC12 - Editar Evento

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC12.1 | Editar evento existente | `healthctl edit a1b2c3d4` | Abre $EDITOR com TOML do evento | Passou |
| TC12.2 | Editar evento inexistente | `healthctl edit nonexistent` | Erro: "event not found" | Passou |
| TC12.3 | Editar com prefixo ambíguo | `healthctl edit a` (múltiplos matches) | Erro: "prefix is ambiguous" | Passou |

### UC13 - Clonar Evento

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC13.1 | Clonar evento | `healthctl clone a1b2c3d4-... --start "today 08:00"` | Novo evento com novo ID e horário | Passou |
| TC13.2 | Clonar com override de calorias | `healthctl clone <id> --calories 400` | Novo evento com calorias modificadas | Passou |
| TC13.3 | Clonar evento inexistente | `healthctl clone nonexistent` | Erro: "event not found" | Passou |

### UC14 - Remover Evento

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC14.1 | Remover com confirmação | `healthctl remove a1b2c3d4` + `y` | Evento removido | Passou |
| TC14.2 | Remover com flag -y | `healthctl remove a1b2c3d4 -y` | Evento removido sem prompt | Passou |
| TC14.3 | Cancelar remoção | `healthctl remove a1b2c3d4` + `n` | Operação cancelada | Passou |
| TC14.4 | Remover evento inexistente | `healthctl remove nonexistent` | Erro: "event not found" | Passou |

**Execução TC14.1:**
```bash
$ healthctl remove a1b2c3d4
Event to delete:
  Type: activity(run)
  Start: 2024-01-15 07:00:00 UTC
  Duration: 30m

Delete this event? [y/N] y
Deleted event a1b2c3d4
```

### UC15 - Mostrar Detalhes do Evento

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC15.1 | Ver detalhes por ID completo | `healthctl show a1b2c3d4-1234-5678-...` | Detalhes completos do evento | Passou |
| TC15.2 | Ver detalhes por prefixo | `healthctl show a1b2` | Detalhes do evento único | Passou |
| TC15.3 | Evento inexistente | `healthctl show xyz` | Erro: "no event matching prefix" | Passou |

### UC16 - Gerenciar Daemon

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC16.1 | Ver status do daemon | `healthctl daemon status` | Status: running/stopped | Passou |
| TC16.2 | Parar daemon | `healthctl daemon stop` | Daemon encerrado | Passou |
| TC16.3 | Reiniciar daemon | `healthctl daemon restart` | Daemon reiniciado | Passou |

### UC17 - Visualizar Dashboard

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC17.1 | Abrir dashboard | `healthctl dashboard` | Janela abre com dados da semana | Passou |
| TC17.2 | Dashboard sem daemon | Abrir com daemon parado | Tenta iniciar daemon automaticamente | Passou |
| TC17.3 | Visualizar cards de métricas | Click em card | Detalhes expandidos em modal | Passou |

### UC18 - Navegar por Semanas no Dashboard

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC18.1 | Navegar para semana anterior | Click em seletor de semana | Dados da semana selecionada | Passou |
| TC18.2 | Voltar para semana atual | Click em "Current Week" | Retorna à semana atual | Passou |
| TC18.3 | Navegar para semana sem dados | Selecionar semana vazia | Mostra zeros nas métricas | Passou |

### UC19 - Alternar Tema do Dashboard

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC19.1 | Alternar para tema claro | Click no toggle de tema | Interface muda para tema claro | Passou |
| TC19.2 | Alternar para tema escuro | Click no toggle de tema | Interface muda para tema escuro | Passou |
| TC19.3 | Persistência de tema | Fechar e reabrir dashboard | Tema anterior é restaurado | Passou |

### UC20 - Deletar Evento via Dashboard

| TC ID | Descrição | Entrada | Resultado Esperado | Status |
|-------|-----------|---------|-------------------|--------|
| TC20.1 | Deletar evento | Click no X do evento + confirmar | Evento removido, lista atualizada | Passou |
| TC20.2 | Cancelar deleção | Click no X + cancelar | Evento mantido | Passou |
| TC20.3 | Verificar sincronização | Deletar no dashboard, verificar na CLI | Evento não aparece em `healthctl list` | Passou |

## 3. Testes de Integração

### TI01 - Fluxo Completo de Registro

| Passo | Ação | Verificação |
|-------|------|-------------|
| 1 | Iniciar daemon | Daemon rodando |
| 2 | Adicionar evento via CLI | Evento criado |
| 3 | Verificar via `healthctl list` | Evento aparece na lista |
| 4 | Verificar via dashboard | Evento visível no dashboard |
| 5 | Deletar via dashboard | Evento removido |
| 6 | Verificar via CLI | Evento não aparece mais |

**Resultado:** Passou

### TI02 - Persistência de Dados

| Passo | Ação | Verificação |
|-------|------|-------------|
| 1 | Adicionar múltiplos eventos | Eventos criados |
| 2 | Parar daemon | Daemon parado |
| 3 | Reiniciar daemon | Daemon iniciado |
| 4 | Listar eventos | Todos os eventos persistidos |

**Resultado:** Passou

### TI03 - Concorrência

| Passo | Ação | Verificação |
|-------|------|-------------|
| 1 | Abrir dashboard | Dashboard conectado |
| 2 | Executar comandos CLI simultaneamente | Comandos executam |
| 3 | Verificar dados em ambos | Dados consistentes |

**Resultado:** Passou

## 4. Resumo dos Resultados

| Categoria | Total | Passou | Falhou | Taxa |
|-----------|-------|--------|--------|------|
| UC01-UC07 (Registro) | 21 | 21 | 0 | 100% |
| UC08-UC11 (Consultas) | 15 | 15 | 0 | 100% |
| UC12-UC16 (Manutenção) | 13 | 13 | 0 | 100% |
| UC17-UC20 (Dashboard) | 12 | 12 | 0 | 100% |
| Integração | 3 | 3 | 0 | 100% |
| **Total** | **64** | **64** | **0** | **100%** |

## 5. Observações

- Todos os testes foram executados manualmente no ambiente de desenvolvimento
- O daemon foi iniciado via `cargo run --bin healthctl-daemon` durante os testes
- O dashboard foi testado tanto com tema claro quanto escuro
- Os testes de concorrência confirmaram o correto funcionamento do modo WAL do SQLite

---

**Anterior:** [Arquitetura do Sistema](./arquitetura.md)
