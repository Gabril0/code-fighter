"""Q5: API de Contas.

O servidor, os modelos de dados e as validações já estão prontos.
Falta preencher o corpo das quatro rotas marcadas com TODO.

Rodar:
    uvicorn main:app --reload

Documentação interativa (dá para testar as rotas pelo navegador):
    http://127.0.0.1:8000/docs
"""

from itertools import count
from typing import Literal

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field

app = FastAPI(title="API de Contas")

# Armazenamento em memória. Não precisa de banco de dados.
# Estrutura sugerida para cada conta:
#     {"id": 1, "titular": "Maria Silva", "saldo": 50000, "transacoes": []}
contas = {}

# Gerador de ids: `next(gerador_de_id)` devolve 1, depois 2, depois 3...
# (usar um contador assim evita ter que declarar `global` dentro das funções)
gerador_de_id = count(1)


class ContaNova(BaseModel):
    """Corpo do POST /contas. O Field já cuida do 422 sozinho."""
    titular: str = Field(min_length=1)
    saldo_inicial: int = Field(ge=0)


class TransacaoNova(BaseModel):
    """Corpo do POST /contas/{id}/transacoes."""
    tipo: Literal["CREDITO", "DEBITO"]
    valor: int = Field(gt=0)


def conta_publica(conta: dict) -> dict:
    """A conta como ela deve aparecer na resposta, sem a lista de transações."""
    return {"id": conta["id"], "titular": conta["titular"], "saldo": conta["saldo"]}


@app.post("/contas", status_code=201)
def criar_conta(dados: ContaNova):
    # TODO: montar a conta com um id novo, guardar em `contas` e devolvê-la.
    #       Lembre que ela nasce com a lista de transações vazia.
    raise NotImplementedError


@app.get("/contas/{conta_id}")
def buscar_conta(conta_id: int):
    # TODO: devolver a conta.
    #       Se ela não existir:
    #           raise HTTPException(status_code=404, detail="conta nao encontrada")
    raise NotImplementedError


@app.post("/contas/{conta_id}/transacoes")
def criar_transacao(conta_id: int, dados: TransacaoNova):
    # TODO: 404 se a conta não existir.
    #       CREDITO soma ao saldo. DEBITO subtrai, mas só se houver saldo:
    #       senão 400 com detail "saldo insuficiente", sem alterar nada.
    #       Registre a transação aplicada no extrato e devolva a conta atualizada.
    raise NotImplementedError


@app.get("/contas/{conta_id}/extrato")
def buscar_extrato(conta_id: int):
    # TODO: devolver {"conta_id": ..., "transacoes": [...]}, ou 404.
    raise NotImplementedError
