FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY target/release/evlogstudio .

RUN chmod +x evlogstudio

EXPOSE 8080

ENV PORT=8080
ENV HOST=0.0.0.0
ENV STORAGE_MODE=local
ENV DATA_PATH=/data/logs.duckdb

VOLUME ["/data"]

CMD ["./evlogstudio"]
