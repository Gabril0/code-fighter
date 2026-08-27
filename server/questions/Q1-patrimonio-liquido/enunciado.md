# Q1: Fechamento do Patrimônio Líquido

**Dificuldade:** aquecimento

## Contexto

Todo dia, ao fechar o mercado, um fundo precisa calcular seu **Patrimônio Líquido (PL)**, o quanto o fundo realmente vale. A conta é simples:

```
PL = (soma de tudo que o fundo tem)  −  (soma de tudo que o fundo deve)
```

O que o fundo **tem** são os _ativos_ da carteira: títulos, debêntures, CDBs. Cada posição vale `quantidade × preço unitário`.
O que o fundo **deve** são os _passivos_: taxa de administração a pagar, resgates pendentes de liquidação, etc.

Sua missão é automatizar esse fechamento.

## Entrada

```
N                              → número de ativos na carteira
<nome> <quantidade> <preço>    → N linhas, uma por ativo
M                              → número de passivos
<descrição> <valor>            → M linhas, uma por passivo
```

- `nome` e `descrição` são textos **sem espaços** (você não precisa usá-los no cálculo)
- `quantidade` é um inteiro
- `preço` e `valor` usam **ponto** como separador decimal
- `1 ≤ N ≤ 1000`, `0 ≤ M ≤ 100`

## Saída

Uma única linha com o PL, com **exatamente 2 casas decimais**, ponto como separador decimal e **sem separador de milhar**.

## Exemplo 1

**Entrada**

```
3
LFT-2029 1500 14320.50
DEB-XPTO21 800 1050.00
CDB-BANCOX 200 1230.75
2
TAXA_ADMIN 12500.00
RESGATES_A_PAGAR 340000.00
```

**Saída**

```
22214400.00
```

> 1500 × 14320,50 = 21.480.750,00
> 800 × 1050,00 = 840.000,00
> 200 × 1230,75 = 246.150,00
> Ativos = 22.566.900,00 · Passivos = 352.500,00 · **PL = 22.214.400,00**

## Exemplo 2

**Entrada**

```
1
CDB-BANCOY 10 100.00
0
```

**Saída**

```
1000.00
```

## Observações

- Um fundo pode não ter nenhum passivo (`M = 0`).
- O PL pode ser negativo: acontece na vida real, com fundos alavancados.
