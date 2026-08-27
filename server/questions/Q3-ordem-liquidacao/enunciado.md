# Q3: Ordem de Liquidação

**Dificuldade:** intermediária

## Contexto

Uma conta digital recebe transações o dia todo por filas e webhooks, e elas **chegam fora de ordem**. Antes de fechar o dia, o sistema precisa colocar tudo na ordem correta de liquidação e só então aplicar os lançamentos.

A ordem importa de verdade: se um débito for processado antes do crédito que o cobre, ele é rejeitado por saldo insuficiente, e o cliente leva uma recusa que não deveria ter existido. É por isso que todo banco credita antes de debitar.

Todos os valores estão em **centavos**, como inteiros. Sistema financeiro não usa número com vírgula: `0,1 + 0,2` não dá exatamente `0,3` em ponto flutuante, e um centavo perdido numa conta bancária é um problema contábil de verdade. Trabalhe só com inteiros e nada vai arredondar sozinho.

## Regras de ordenação

Ordene as transações assim, nesta prioridade:

1. **`TARIFA` vem sempre depois de todas as outras**, não importa o horário: tarifas são lançadas no fechamento do dia.
2. Dentro de cada grupo, por **timestamp crescente**.
3. Empatou no timestamp: **`CREDITO` antes de `DEBITO`**.
4. Empatou ainda: por **id crescente**, comparado como texto.

## Regras de processamento

Depois de ordenar, aplique na ordem:

- **`CREDITO`**: soma o valor ao saldo. Nunca é rejeitado.
- **`DEBITO`** e **`TARIFA`**: subtraem o valor do saldo, mas **só se o saldo atual for maior ou igual ao valor**. Se não for, a transação é **rejeitada** e o saldo não muda.

## Entrada

```
<saldo_inicial>                     → inteiro, em centavos
<id>;<timestamp>;<tipo>;<valor>     → uma linha por transação, até o fim da entrada
```

- `timestamp` no formato `AAAA-MM-DDTHH:MM:SS`, sempre com zeros à esquerda.
- `tipo` é `CREDITO`, `DEBITO` ou `TARIFA`.
- `valor` é um inteiro positivo, em centavos.
- `id` é único e todos os ids têm o mesmo comprimento.
- `0 ≤ saldo_inicial ≤ 10^11`, até `50000` transações, `1 ≤ valor ≤ 10^9`

## Saída

```
Linha 1: saldo final, em centavos
Linha 2: menor saldo atingido em qualquer momento, em centavos
Linha 3: quantidade de transações rejeitadas
Linhas seguintes: o id de cada transação rejeitada, na ordem em que foram processadas
```

O **menor saldo** considera também o saldo inicial. Se nada foi rejeitado, a linha 3 é `0` e não vem nenhuma linha depois dela.

## Exemplo 1

**Entrada**

```
10000
TXN0003;2026-03-14T09:00:00;DEBITO;15000
TXN0001;2026-03-14T09:00:00;CREDITO;20000
TXN0002;2026-03-14T10:30:00;DEBITO;12000
```

**Saída**

```
3000
3000
0
```

**Passo a passo**

TXN0003 e TXN0001 têm o mesmo horário, então o crédito vem primeiro. Ordem de liquidação: `TXN0001 → TXN0003 → TXN0002`.

| Transação | Efeito | Saldo |
|---|---|---|
| - | saldo inicial | 10.000 |
| TXN0001 `CREDITO` 20.000 | soma | 30.000 |
| TXN0003 `DEBITO` 15.000 | subtrai | 15.000 |
| TXN0002 `DEBITO` 12.000 | subtrai | **3.000** |

> Repare no que aconteceria se você processasse na ordem do arquivo: TXN0003 chegaria primeiro, com saldo de apenas 10.000, e seria **rejeitado**. A resposta sairia `18000 / 10000 / 1`, completamente diferente. Ordenar não é detalhe, é a questão.

## Exemplo 2

**Entrada**

```
5000
TXN0002;2026-03-14T08:00:00;DEBITO;7000
TXN0005;2026-03-14T07:00:00;TARIFA;500
TXN0001;2026-03-14T08:00:00;DEBITO;3000
TXN0004;2026-03-14T12:00:00;CREDITO;10000
TXN0003;2026-03-14T12:00:00;DEBITO;9000
```

**Saída**

```
2500
2000
1
TXN0002
```

**Passo a passo**

TXN0005 é a transação com o horário **mais antigo** do arquivo (07:00), mas é uma `TARIFA`, então vai para o fim da fila. Às 12:00 o crédito TXN0004 passa na frente do débito TXN0003, mesmo tendo id maior. Ordem de liquidação: `TXN0001 → TXN0002 → TXN0004 → TXN0003 → TXN0005`.

| Transação | Efeito | Saldo |
|---|---|---|
| - | saldo inicial | 5.000 |
| TXN0001 `DEBITO` 3.000 | subtrai | 2.000 |
| TXN0002 `DEBITO` 7.000 | **rejeitada**: 2.000 < 7.000 | 2.000 |
| TXN0004 `CREDITO` 10.000 | soma | 12.000 |
| TXN0003 `DEBITO` 9.000 | subtrai | 3.000 |
| TXN0005 `TARIFA` 500 | subtrai | **2.500** |

Menor saldo atingido: **2.000**.

## Observações

- Uma `TARIFA` também pode ser rejeitada por saldo insuficiente.
- O saldo nunca fica negativo: é justamente isso que a rejeição garante.
- Pode não haver nenhuma transação rejeitada, e pode haver dias em que todas são.
