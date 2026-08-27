# Q5: API de Contas

**Dificuldade:** aplicada

## Contexto

Toda a prova até aqui foi processamento em lote: entra arquivo, sai resposta. Na vida real, esse mesmo domínio vive atrás de uma API: é assim que o resto da empresa consulta saldo e movimenta conta.

Sua missão é implementar uma API de contas bem pequena. Você recebe um **esqueleto pronto** em FastAPI, com o servidor configurado e os modelos de dados já escritos. Só falta preencher o corpo de quatro rotas.

Como na Q3, **todos os valores são inteiros em centavos**. Nada de número com vírgula.

## Como começar

Baixe o pacote desta questão pelo botão **Baixar esqueleto** no painel de questões. Dentro dele vêm o esqueleto em `skeleton/`, o `teste_api.py` e o `COMO_ENVIAR.md`.

As instruções para rodar estão em `skeleton/COMO_RODAR.md`. Não precisa de banco de dados: os dados ficam em memória e somem quando o servidor reinicia, e tudo bem.

Você pode usar outro framework ou outra linguagem se preferir, desde que o contrato abaixo seja respeitado à risca. Mas o esqueleto existe justamente para você não perder tempo com isso.

## Contrato

### `POST /contas`: cria uma conta

Corpo:

```json
{"titular": "Maria Silva", "saldo_inicial": 50000}
```

| Situação | Status | Resposta |
|---|---|---|
| Criada | `201` | `{"id": 1, "titular": "Maria Silva", "saldo": 50000}` |
| `saldo_inicial` negativo, ou `titular` vazio | `422` | (o FastAPI devolve sozinho) |

O `id` é gerado pelo servidor e começa em `1`.

### `GET /contas/{conta_id}`: consulta uma conta

| Situação | Status | Resposta |
|---|---|---|
| Existe | `200` | `{"id": 1, "titular": "Maria Silva", "saldo": 50000}` |
| Não existe | `404` | `{"detail": "conta nao encontrada"}` |

### `POST /contas/{conta_id}/transacoes`: movimenta a conta

Corpo:

```json
{"tipo": "CREDITO", "valor": 15000}
```

`tipo` é `CREDITO` (soma) ou `DEBITO` (subtrai).

| Situação | Status | Resposta |
|---|---|---|
| Aplicada | `200` | a conta atualizada: `{"id": 1, "titular": "...", "saldo": 65000}` |
| Conta não existe | `404` | `{"detail": "conta nao encontrada"}` |
| `DEBITO` maior que o saldo | `400` | `{"detail": "saldo insuficiente"}` |
| `valor` zero ou negativo, `tipo` inválido | `422` | (o FastAPI devolve sozinho) |

Um débito recusado **não altera o saldo e não entra no extrato**.

### `GET /contas/{conta_id}/extrato`: lista as transações

| Situação | Status | Resposta |
|---|---|---|
| Existe | `200` | `{"conta_id": 1, "transacoes": [{"tipo": "CREDITO", "valor": 15000}]}` |
| Não existe | `404` | `{"detail": "conta nao encontrada"}` |

As transações vêm na ordem em que foram aplicadas. Conta recém-criada tem extrato vazio: `{"conta_id": 1, "transacoes": []}`.

## Conferindo antes de enviar

Com o servidor rodando, em outro terminal, a partir da pasta do pacote:

```bash
python3 teste_api.py http://127.0.0.1:8000
```

O script bate na sua API e imprime um `PASSOU` ou `FALHOU` para cada item do contrato, com o placar no fim. Ele usa só a biblioteca padrão do Python, não precisa instalar nada nem ativar o venv. Pode rodar quantas vezes quiser, sem reiniciar o servidor.

São **28 checagens**. Antes de você escrever qualquer linha, 5 já passam sozinhas: são as de validação (`422`), que o Pydantic resolve antes de chegar no seu código. As outras 23 são com você.

## Como enviar

Esta questão não tem arquivo de saída para anexar. **O juiz roda as mesmas 28 checagens contra a sua API no ar**, então ele precisa alcançar o seu servidor pela internet.

1. Deixe a sua API rodando localmente.
2. Abra um túnel em outro terminal, que devolve um endereço público:

   ```bash
   cloudflared tunnel --url http://localhost:8000
   ```

   Ou, se preferir o ngrok: `ngrok http 8000`.

3. Cole o endereço `https://` que o túnel imprimiu no campo **Endereço da sua API** no painel de questões e clique em **Verificar API**.

Colar `http://localhost:8000` ou o IP da sua máquina na rede **não funciona**: o placar roda na internet e não enxerga a sua máquina. Deixe o túnel aberto até o juiz aceitar, porque se ele cair o endereço muda.

As 28 checagens precisam passar todas. Errar não tira ponto, pode tentar quantas vezes quiser. O passo a passo completo, com o que fazer quando dá errado, está no `COMO_ENVIAR.md` do pacote.

## Observações

- Contas diferentes são independentes: mexer numa não pode afetar a outra.
- Não precisa se preocupar com autenticação, concorrência ou persistência.
- O saldo nunca fica negativo: é o `400` que garante isso.
