FROM python:3.12-slim-bookworm
WORKDIR /app
ENV PYTHONUNBUFFERED=1 PIP_NO_CACHE_DIR=1
COPY requirements.txt ./
RUN pip install --only-binary=:all: -r requirements.txt
COPY service.py ./
RUN mkdir -p /app/.cache && chown -R 65532:65532 /app
USER 65532:65532
EXPOSE 3002
CMD ["python", "service.py"]
