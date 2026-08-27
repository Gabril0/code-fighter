# Q4: Fechamento do Fundo

**Dificuldade:** intermediária+

## Contexto

Um fundo funciona como um bolo que cresce sozinho e do qual as pessoas entram e saem.

- Uma **integralização** é dinheiro entrando: um investidor aporta um valor e passa a ter saldo no fundo.
- Uma **amortização** é dinheiro saindo: o fundo devolve um valor aos investidores, **proporcionalmente ao saldo de cada um**.
- Todo dia, o saldo de todos os investidores rende a mesma taxa.

Repare no detalhe que faz a coisa toda funcionar: quem entrou antes tem saldo maior no momento da amortização, então recebe uma fatia maior. É por isso que a proporção tem que ser calculada sobre o **saldo corrigido**, não sobre o valor que a pessoa aportou.

Você vai receber o log de movimentações do fundo e precisa dizer, no fim do processo, quanto cada investidor colocou, quanto recebeu de volta e quanto lucrou.

## Regras de processamento

1. Todos os saldos começam em zero.
2. Os eventos vêm em ordem cronológica. **Processe-os exatamente na ordem em que aparecem.**
3. Antes de processar um evento do dia `D`, aplique o rendimento acumulado desde o dia do último evento processado: multiplique o saldo de **todos** os investidores por `(1 + taxa) ^ (D − último dia)`.
   - Consequência: se vários eventos acontecem no mesmo dia, o rendimento é aplicado **uma vez só**, antes do primeiro deles.
   - Consequência: dinheiro que entra no dia `D` não rende no dia `D`.
4. **`INTEGRALIZACAO`**: soma o valor ao saldo daquele investidor.
5. **`AMORTIZACAO`** de valor `A`: cada investidor recebe `A × (saldo dele ÷ saldo total do fundo)`, e esse mesmo valor é descontado do saldo dele.
6. **`AMORTIZACAO_TOTAL`**: cada investidor recebe todo o seu saldo, que vai a zero. É sempre o último evento do log.
7. **Não arredonde nada no meio do caminho.** Arredonde só na hora de montar a resposta.

## Entrada

```
<taxa>                              → taxa de rendimento diária, em fração
<dia>;<tipo>;<investidor>;<valor>   → uma linha por evento, até o fim da entrada
```

- A **taxa** vem com 6 casas decimais, em fração: `0.001000` significa 0,1% ao dia.
- Cada evento tem sempre **4 campos** separados por `;`, mas alguns ficam vazios:
  - `INTEGRALIZACAO` → `1;INTEGRALIZACAO;INV001;100000.00`
  - `AMORTIZACAO` → `3;AMORTIZACAO;;30000.00` (sem investidor)
  - `AMORTIZACAO_TOTAL` → `5;AMORTIZACAO_TOTAL;;` (sem investidor e sem valor)
- `investidor` é um identificador em `A-Z` e `0-9`, sem espaços.
- Os dias são inteiros e **não decrescentes**.
- Uma `AMORTIZACAO` nunca pede mais do que o saldo total do fundo.
- `1 ≤ dia ≤ 100000`, até `5000` eventos, até `500` investidores, `0 ≤ taxa ≤ 0.01`
- Cada valor está entre `0.01` e `10^9`, e o saldo total do fundo nunca ultrapassa `10^12`.

## Saída

Um objeto JSON com uma chave por investidor, cada uma contendo:

- `total_integralizado`: soma de tudo que ele aportou
- `total_recebido`: soma de tudo que ele recebeu em amortizações
- `resultado`: `total_recebido − total_integralizado`

Todos os valores arredondados para **2 casas decimais**. A ordem das chaves não importa, e a formatação do JSON (espaços, quebras de linha) também não: a correção lê o JSON, não o texto.

## Exemplo 1

**Entrada**

```
0.001000
1;INTEGRALIZACAO;INV001;100000.00
1;INTEGRALIZACAO;INV002;50000.00
3;AMORTIZACAO;;30000.00
5;AMORTIZACAO_TOTAL;;
```

**Saída**

```json
{
  "INV001": {"total_integralizado": 100000.00, "total_recebido": 100360.58, "resultado": 360.58},
  "INV002": {"total_integralizado": 50000.00, "total_recebido": 50180.29, "resultado": 180.29}
}
```

**Passo a passo**

| Momento | INV001 | INV002 | O que aconteceu |
|---|---|---|---|
| Dia 1 | 100.000,00 | 50.000,00 | Duas integralizações. Saldos zerados antes, rendimento irrelevante. |
| Dia 3, antes do evento | 100.200,10 | 50.100,05 | 2 dias de rendimento: `× 1,001² = 1,002001` |
| Dia 3, amortização de 30.000 | recebe **20.000,00** → sobra 80.200,10 | recebe **10.000,00** → sobra 40.100,05 | Saldo total = 150.300,15. INV001 tem 2/3 dele, INV002 tem 1/3. |
| Dia 5, antes do evento | 80.360,58… | 40.180,29… | 2 dias de rendimento: `× 1,002001` |
| Dia 5, amortização total | recebe **80.360,58** | recebe **40.180,29** | Saldos vão a zero. |

Somando as amortizações: INV001 recebeu 100.360,58 sobre 100.000,00 aportados → lucro de **360,58**. INV002 recebeu 50.180,29 sobre 50.000,00 → lucro de **180,29**. Como INV002 entrou com metade e no mesmo dia, o lucro dele é exatamente metade, bom sinal de que a conta está certa.

## Exemplo 2

**Entrada**

```
0.001000
1;INTEGRALIZACAO;INV001;100000.00
3;AMORTIZACAO;;50000.00
3;INTEGRALIZACAO;INV002;50000.00
6;AMORTIZACAO_TOTAL;;
```

**Saída**

```json
{
  "INV001": {"total_integralizado": 100000.00, "total_recebido": 100350.85, "resultado": 350.85},
  "INV002": {"total_integralizado": 50000.00, "total_recebido": 50150.15, "resultado": 150.15}
}
```

**Passo a passo**

No dia 3 acontecem duas coisas, e **a ordem importa**: primeiro o fundo rende 2 dias (INV001 vai a 100.200,10), depois vem a amortização de 50.000: com INV002 ainda fora do fundo, INV001 leva os 50.000 inteiros e fica com 50.200,10. **Só então** INV002 integraliza 50.000. Note que INV002 não participou daquela amortização, e nem rendeu naqueles 2 primeiros dias.

Do dia 3 ao 6 são 3 dias de rendimento (`× 1,001³`), e a amortização total devolve o que sobrou.

## Observações

- Um investidor pode integralizar mais de uma vez.
- Um investidor pode entrar no fundo depois de amortizações já terem acontecido.
- Se a taxa for `0.000000`, ninguém lucra nada e o `resultado` de todos é `0.00`.
- Os dias vão até 100.000, mas os eventos são no máximo 5.000, ou seja, há **buracos grandes** entre eventos. Pense nisso antes de escrever um laço que percorre dia a dia.
