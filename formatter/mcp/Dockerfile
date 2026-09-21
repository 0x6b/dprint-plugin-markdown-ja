FROM rust:1.92-bookworm@sha256:e90e846de4124376164ddfbaab4b0774c7bdeef5e738866295e5a90a34a307a2 AS builder

WORKDIR /src
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --locked -p dprint-markdown-ja-formatter-mcp --profile formatter-release \
    && python3 formatter/scripts/license-notices.py dprint-markdown-ja-formatter-mcp

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f

COPY --from=builder /src/target/formatter-release/dprint-markdown-ja-formatter-mcp /usr/local/bin/dprint-markdown-ja-formatter-mcp
COPY --from=builder /src/target/THIRD_PARTY_NOTICES.md /licenses/dprint-markdown-ja-formatter/THIRD_PARTY_NOTICES.md
COPY LICENSE /licenses/dprint-markdown-ja-formatter/LICENSE

EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/dprint-markdown-ja-formatter-mcp"]
CMD ["--listen", "0.0.0.0:3000", "--allowed-host", "localhost", "--allowed-host", "localhost:3000"]
