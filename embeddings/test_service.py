"""Tests against the real embedding model (downloaded on first run)."""
import json
import threading
import urllib.request
import urllib.error
import pytest


@pytest.fixture(scope='module')
def server():
    import service
    assert hasattr(service, 'make_server'), 'HTTP service is not implemented'
    httpd = service.make_server(port=0)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    yield 'http://127.0.0.1:' + str(httpd.server_port)
    httpd.shutdown()
    httpd.server_close()
    thread.join()


def post(server, payload):
    req = urllib.request.Request(server + '/v1/embeddings',
        data=json.dumps(payload).encode(), headers={'Content-Type': 'application/json'})
    try:
        with urllib.request.urlopen(req) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, json.load(error)


@pytest.mark.parametrize('payload, status', [
    ({'input': ''}, 400), ({'input': '   '}, 400), ({'input': []}, 400),
    ({'input': [17]}, 400), ({'input': 'x', 'model': 'wrong'}, 400),
    ({'input': ['x'] * 33}, 400), ({'input': 'x' * 8193}, 400),
    ({'input': ['x' * 8192] * 32}, 413),
    ({}, 400), ([], 400), ({'input': None}, 400),
])
def test_rejects_invalid_inputs(server, payload, status):
    actual, body = post(server, payload)
    assert actual == status
    assert 'error' in body


def test_http_contract(server):
    with urllib.request.urlopen(server + '/health') as response:
        health = json.load(response)
    assert health['model'] == 'BAAI/bge-small-en-v1.5'
    assert health['dimension'] == 384
    single_status, single = post(server, {'input': 'Manassas food pantry'})
    batch_status, batch = post(server, {'input': ['Manassas food pantry']})
    assert single_status == batch_status == 200
    assert single['data'][0]['embedding'] == batch['data'][0]['embedding']
    assert len(single['data'][0]['embedding']) == 384
import numpy as np


def test_socket_timeout():
    import service
    assert service.Handler.timeout == 5


def test_malformed_json(server):
    req = urllib.request.Request(server + '/v1/embeddings', data=b'{',
                                 headers={'Content-Type': 'application/json'})
    with pytest.raises(urllib.error.HTTPError) as error:
        urllib.request.urlopen(req)
    assert error.value.code == 400


def test_entrypoint_exists():
    import service
    assert callable(getattr(service, 'main', None))


def test_real_embeddings():
    from service import Engine, MODEL
    engine = Engine()
    result = engine.embed(['food pantry groceries', 'food pantry groceries', 'quantum physics research'])
    vectors = np.array([row['embedding'] for row in result['data']])
    assert result['model'] == MODEL == 'BAAI/bge-small-en-v1.5'
    assert vectors.shape == (3, 384)
    assert np.isfinite(vectors).all()
    assert np.allclose(np.linalg.norm(vectors, axis=1), 1, atol=1e-5)
    assert np.allclose(vectors[0], vectors[1], atol=1e-6)
    assert np.dot(vectors[0], vectors[1]) > np.dot(vectors[0], vectors[2]) + 0.1
    assert [row['index'] for row in result['data']] == [0, 1, 2]
