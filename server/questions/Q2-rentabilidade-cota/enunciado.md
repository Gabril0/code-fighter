# Q2: Rentabilidade da Cota

**Dificuldade:** fácil

## Contexto

O cotista não acompanha o PL do fundo; ele acompanha o **valor da cota**. É esse número, divulgado todo dia, que diz se o dinheiro dele rendeu ou não.

A **rentabilidade acumulada** de um período compara a cota do último dia com a do primeiro:

```
rentabilidade acumulada (%) = (cota final ÷ cota inicial − 1) × 100
```

E a **variação diária** compara cada dia com o dia anterior:

```
variação do dia D (%) = (cota do dia D ÷ cota do dia D−1 − 1) × 100
```

O time de produto quer publicar dois números na lâmina do fundo: quanto ele rendeu no período, e qual foi o melhor dia.

## Entrada

```
N               → quantidade de dias
<valor>         → N linhas, o valor da cota em cada dia, do dia 1 ao dia N
```

- O valor da cota vem com 6 casas decimais e ponto como separador decimal.
- `2 ≤ N ≤ 1000`

## Saída

Duas linhas:

1. A **rentabilidade acumulada** do período, em %, com exatamente 2 casas decimais.
2. O **melhor dia** e a **variação desse dia**, separados por um espaço, sendo "melhor dia" aquele com a maior variação diária, com a variação também em % e 2 casas decimais.

Se dois dias empatarem na maior variação, vence o dia menor. Se o fundo só caiu no período, o melhor dia é simplesmente o de menor queda.

## Exemplo 1

**Entrada**

```
4
100.000000
102.000000
101.000000
105.000000
```

**Saída**

```
5.00
4 3.96
```

> Acumulada = (105 ÷ 100 − 1) × 100 = **5,00%**
> Dia 2 = (102 ÷ 100 − 1) × 100 = +2,00%
> Dia 3 = (101 ÷ 102 − 1) × 100 = −0,98%
> Dia 4 = (105 ÷ 101 − 1) × 100 = **+3,96%** ← melhor dia

## Exemplo 2

**Entrada**

```
4
200.000000
190.000000
189.000000
170.000000
```

**Saída**

```
-15.00
3 -0.53
```

> Acumulada = (170 ÷ 200 − 1) × 100 = **−15,00%**
> Dia 2 = −5,00% · Dia 3 = **−0,53%** · Dia 4 = −10,05%
> O fundo caiu todos os dias, então o melhor dia é o de menor queda: **dia 3**

## Observações

- Atenção: a rentabilidade acumulada **não é a soma** das variações diárias. Confira no Exemplo 1: a soma daria 4,98%, e a resposta certa é 5,00%. Rentabilidade se multiplica, não se soma.
- O dia 1 não tem variação diária (não existe dia anterior), então o melhor dia é sempre algum entre 2 e N.
