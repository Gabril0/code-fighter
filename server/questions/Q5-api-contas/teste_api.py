"""Bateria de testes da Q5: roda contra a sua API no ar.

Suba o servidor num terminal e rode isto em outro:

    python3 teste_api.py                          # usa http://127.0.0.1:8000
    python3 teste_api.py http://127.0.0.1:8001    # outra porta

Usa só a biblioteca padrão, não precisa instalar nada.
"""

import json
import sys
import urllib.error
import urllib.request

BASE = sys.argv[1].rstrip("/") if len(sys.argv) > 1 else "http://127.0.0.1:8000"

passou = 0
falhou = 0


def chamar(metodo: str, caminho: str, corpo=None):
    """Devolve (status, json_decodificado). Não levanta exceção em erro HTTP."""
    dados = json.dumps(corpo).encode() if corpo is not None else None
    req = urllib.request.Request(BASE + caminho, data=dados, method=metodo)
    if dados:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req) as resp:
            texto = resp.read().decode()
            return resp.status, (json.loads(texto) if texto else None)
    except urllib.error.HTTPError as erro:
        texto = erro.read().decode()
        try:
            return erro.code, json.loads(texto)
        except json.JSONDecodeError:
            return erro.code, texto
    except urllib.error.URLError as erro:
        print(f"\nNão consegui falar com {BASE}. O servidor está no ar?")
        print(f"  ({erro.reason})")
        sys.exit(1)


def checar(descricao: str, condicao: bool, obtido=None) -> None:
    global passou, falhou
    if condicao:
        passou += 1
        print(f"  PASSOU  {descricao}")
    else:
        falhou += 1
        print(f"  FALHOU  {descricao}")
        if obtido is not None:
            print(f"          recebi: {obtido}")


print(f"Testando {BASE}\n")

# --- POST /contas -----------------------------------------------------------
print("POST /contas")
status, conta = chamar("POST", "/contas", {"titular": "Maria Silva", "saldo_inicial": 50000})
checar("cria conta e devolve 201", status == 201, status)
checar("resposta traz id, titular e saldo",
       isinstance(conta, dict) and {"id", "titular", "saldo"} <= set(conta), conta)
checar("saldo inicial é respeitado", isinstance(conta, dict) and conta.get("saldo") == 50000, conta)

id_maria = conta.get("id") if isinstance(conta, dict) else None

status, _ = chamar("POST", "/contas", {"titular": "Joao Souza", "saldo_inicial": -1})
checar("saldo_inicial negativo devolve 422", status == 422, status)

status, _ = chamar("POST", "/contas", {"titular": "", "saldo_inicial": 100})
checar("titular vazio devolve 422", status == 422, status)

# --- GET /contas/{id} -------------------------------------------------------
print("\nGET /contas/{id}")
status, conta = chamar("GET", f"/contas/{id_maria}")
checar("consulta conta existente devolve 200", status == 200, status)
checar("dados batem com o que foi criado",
       isinstance(conta, dict) and conta.get("titular") == "Maria Silva"
       and conta.get("saldo") == 50000, conta)

status, corpo = chamar("GET", "/contas/99999")
checar("conta inexistente devolve 404", status == 404, status)
checar("404 traz detail 'conta nao encontrada'",
       isinstance(corpo, dict) and corpo.get("detail") == "conta nao encontrada", corpo)

# --- POST /contas/{id}/transacoes -------------------------------------------
print("\nPOST /contas/{id}/transacoes")
status, conta = chamar("POST", f"/contas/{id_maria}/transacoes", {"tipo": "CREDITO", "valor": 15000})
checar("crédito devolve 200", status == 200, status)
checar("crédito soma ao saldo (50000 + 15000 = 65000)",
       isinstance(conta, dict) and conta.get("saldo") == 65000, conta)

status, conta = chamar("POST", f"/contas/{id_maria}/transacoes", {"tipo": "DEBITO", "valor": 5000})
checar("débito devolve 200", status == 200, status)
checar("débito subtrai do saldo (65000 - 5000 = 60000)",
       isinstance(conta, dict) and conta.get("saldo") == 60000, conta)

status, corpo = chamar("POST", f"/contas/{id_maria}/transacoes", {"tipo": "DEBITO", "valor": 999999})
checar("débito maior que o saldo devolve 400", status == 400, status)
checar("400 traz detail 'saldo insuficiente'",
       isinstance(corpo, dict) and corpo.get("detail") == "saldo insuficiente", corpo)

status, conta = chamar("GET", f"/contas/{id_maria}")
checar("débito recusado não alterou o saldo",
       isinstance(conta, dict) and conta.get("saldo") == 60000, conta)

status, conta = chamar("POST", f"/contas/{id_maria}/transacoes", {"tipo": "DEBITO", "valor": 60000})
checar("débito exatamente igual ao saldo é aceito",
       status == 200 and isinstance(conta, dict) and conta.get("saldo") == 0, (status, conta))

chamar("POST", f"/contas/{id_maria}/transacoes", {"tipo": "CREDITO", "valor": 60000})

status, _ = chamar("POST", "/contas/99999/transacoes", {"tipo": "CREDITO", "valor": 100})
checar("transação em conta inexistente devolve 404", status == 404, status)

status, _ = chamar("POST", f"/contas/{id_maria}/transacoes", {"tipo": "CREDITO", "valor": 0})
checar("valor zero devolve 422", status == 422, status)

status, _ = chamar("POST", f"/contas/{id_maria}/transacoes", {"tipo": "CREDITO", "valor": -50})
checar("valor negativo devolve 422", status == 422, status)

status, _ = chamar("POST", f"/contas/{id_maria}/transacoes", {"tipo": "PIX", "valor": 100})
checar("tipo inválido devolve 422", status == 422, status)

# --- GET /contas/{id}/extrato -----------------------------------------------
print("\nGET /contas/{id}/extrato")
status, extrato = chamar("GET", f"/contas/{id_maria}/extrato")
checar("extrato devolve 200", status == 200, status)
checar("extrato traz conta_id e transacoes",
       isinstance(extrato, dict) and {"conta_id", "transacoes"} <= set(extrato), extrato)

esperado = [
    {"tipo": "CREDITO", "valor": 15000},
    {"tipo": "DEBITO", "valor": 5000},
    {"tipo": "DEBITO", "valor": 60000},
    {"tipo": "CREDITO", "valor": 60000},
]
obtido = extrato.get("transacoes") if isinstance(extrato, dict) else None
checar("extrato tem só as 4 transações aceitas, na ordem certa", obtido == esperado, obtido)

status, _ = chamar("GET", "/contas/99999/extrato")
checar("extrato de conta inexistente devolve 404", status == 404, status)

# --- isolamento entre contas ------------------------------------------------
print("\nIsolamento entre contas")
_, outra = chamar("POST", "/contas", {"titular": "Joao Souza", "saldo_inicial": 700})
id_joao = outra.get("id") if isinstance(outra, dict) else None
checar("segunda conta recebe id diferente da primeira", id_joao != id_maria, (id_maria, id_joao))

status, extrato = chamar("GET", f"/contas/{id_joao}/extrato")
checar("conta nova nasce com extrato vazio",
       isinstance(extrato, dict) and extrato.get("transacoes") == [], extrato)

chamar("POST", f"/contas/{id_joao}/transacoes", {"tipo": "DEBITO", "valor": 700})
_, conta = chamar("GET", f"/contas/{id_maria}")
checar("mexer numa conta não afeta a outra",
       isinstance(conta, dict) and conta.get("saldo") == 60000, conta)

# --- placar -----------------------------------------------------------------
total = passou + falhou
print(f"\n{'=' * 46}")
print(f"{passou} de {total} testes passaram.")
if falhou:
    print(f"{falhou} falhando.")
    sys.exit(1)
print("Tudo certo!")
