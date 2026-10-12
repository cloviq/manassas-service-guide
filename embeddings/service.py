"""Local CPU ONNX embeddings; document/query preprocessing is identical."""
import os
from pathlib import Path

# Set before importing native libraries. No worker processes or tokenizer pool.
os.environ['OMP_NUM_THREADS'] = '2'
os.environ['OPENBLAS_NUM_THREADS'] = '2'
os.environ['MKL_NUM_THREADS'] = '2'
os.environ['TOKENIZERS_PARALLELISM'] = 'false'
os.environ['HF_HOME'] = str(Path(__file__).parent / '.cache' / 'huggingface')

from fastembed import TextEmbedding

MODEL = 'BAAI/bge-small-en-v1.5'
DIMENSION = 384

import json
from http.server import BaseHTTPRequestHandler, HTTPServer


class Handler(BaseHTTPRequestHandler):
    timeout = 5
    def log_message(self, *args):
        pass  # Never log request text, paths, or bodies.

    def reply(self, status, payload):
        body = json.dumps(payload, allow_nan=False).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == '/health':
            self.reply(200, {'status': 'ok', 'model': MODEL, 'dimension': DIMENSION,
                             'threads': THREADS, 'provider': 'CPUExecutionProvider'})
        else:
            self.reply(404, {'error': {'message': 'Not found'}})

    def do_POST(self):
        if self.path != '/v1/embeddings':
            self.reply(404, {'error': {'message': 'Not found'}})
            return
        try:
            if self.headers.get('Transfer-Encoding'):
                raise ValueError('Chunked requests are not supported')
            size = int(self.headers.get('Content-Length', '0'))
            if size > 131072:
                self.reply(413, {'error': {'message': 'Body exceeds 131072 bytes'}})
                return
            if size <= 0:
                raise ValueError('A JSON request body is required')
            if self.headers.get_content_type() != 'application/json':
                raise ValueError('Content-Type must be application/json')
            raw = self.rfile.read(size)
            payload = json.loads(raw)
            if not isinstance(payload, dict):
                raise ValueError('Expected a JSON object')
            if payload.get('model', MODEL) != MODEL:
                raise ValueError('Unsupported model')
            texts = payload.get('input')
            if isinstance(texts, str):
                texts = [texts]
            if not isinstance(texts, list) or not 1 <= len(texts) <= 32:
                raise ValueError('input must be a string or array of 1 to 32 strings')
            if any(not isinstance(t, str) or not t.strip() or len(t) > 8192 for t in texts):
                raise ValueError('Each input must be nonblank and at most 8192 characters')
        except (ValueError, UnicodeError):
            self.reply(400, {'error': {'message': 'Invalid request: JSON object, supported model, 1-32 nonblank strings of at most 8192 characters required', 'type': 'invalid_request_error'}})
            return
        try:
            result = self.server.engine.embed(texts)
        except Exception:
            self.reply(500, {'error': {'message': 'Embedding inference failed'}})
            return
        self.reply(200, result)


def make_server(port=3002):
    # Single request at a time bounds inference memory and avoids model races.
    server = HTTPServer((os.environ.get('BIND_HOST', '127.0.0.1'), port), Handler)
    server.engine = Engine()
    return server
THREADS = 2


class Engine:
    def __init__(self):
        self.model = TextEmbedding(
            model_name=MODEL, cache_dir=str(Path(__file__).parent / '.cache'),
            threads=THREADS, providers=['CPUExecutionProvider'], cuda=False,
        )

    def embed(self, texts):
        # Plain text, no asymmetric retrieval prefixes, FastEmbed L2 normalization.
        vectors = self.model.embed(texts, batch_size=8, parallel=None)
        return {'object': 'list', 'model': MODEL, 'data': [
            {'object': 'embedding', 'index': i, 'embedding': vector.tolist()}
            for i, vector in enumerate(vectors)
        ]}


def main():
    with make_server() as server:
        print(f'Embeddings ready on {server.server_address}; model={MODEL}; dimension={DIMENSION}', flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == '__main__':
    main()
