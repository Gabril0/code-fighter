# Como rodar

Precisa de Python 3.9 ou mais novo.

## 1. Instalar as dependências

Dentro da pasta `skeleton/`:

```bash
python3 -m venv .venv
source .venv/bin/activate          # no Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

## 2. Subir o servidor

```bash
uvicorn main:app --reload
```

A API sobe em `http://127.0.0.1:8000`. O `--reload` reinicia sozinho a cada vez que você salva o arquivo.

## 3. Testar

Abra `http://127.0.0.1:8000/docs` no navegador. O FastAPI gera uma página onde dá para disparar cada rota na mão e ver a resposta: é o jeito mais rápido de conferir enquanto desenvolve.

Pela linha de comando:

```bash
curl -X POST http://127.0.0.1:8000/contas \
  -H "Content-Type: application/json" \
  -d '{"titular": "Maria Silva", "saldo_inicial": 50000}'

curl http://127.0.0.1:8000/contas/1
```

## 4. Rodar a bateria de testes

O `teste_api.py` fica **na pasta de cima**, junto do enunciado. Com o servidor de pé, em **outro terminal**:

```bash
cd skeleton && python3 ../teste_api.py
```

Ele imprime `PASSOU` ou `FALHOU` para cada item do contrato. Não precisa instalar nada nem ativar o venv para rodá-lo, ele usa só a biblioteca padrão.

Antes de você preencher qualquer TODO, o placar é **5 de 28**: os testes de validação (`422`) já passam de graça, porque o Pydantic valida o corpo da requisição antes de chamar a sua função.

## 5. Enviar

Quando as 28 passarem localmente, abra um túnel para a sua API ficar acessível pela internet e cole o endereço `https://` no painel de questões do placar. O juiz roda essas mesmas 28 checagens contra ela. O passo a passo está no `COMO_ENVIAR.md`, na pasta de cima.

## Deu erro?

- **`NotImplementedError`**: é o esperado no começo: as rotas ainda estão vazias, é você que vai preenchê-las.
- **`Address already in use`**: já tem algo na porta 8000. Suba em outra com `uvicorn main:app --reload --port 8001` e rode `python3 ../teste_api.py http://127.0.0.1:8001`.
- **`command not found: uvicorn`**: o venv não está ativo. Rode o `source .venv/bin/activate` de novo.
