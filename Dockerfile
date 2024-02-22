# ------------- build ----------------
FROM clux/muslrust:1.76.0 as builder

RUN mkdir -p /rust && mkdir -p /cargo
WORKDIR /rust

# ソースコードのコピー
COPY Cargo.toml Cargo.lock /rust/
COPY src /rust/src

# バイナリ生成
RUN --mount=type=cache,target=/rust/target \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cargo build --release --bin rust-web-tools-test && \
    cp /rust/target/x86_64-unknown-linux-musl/release/rust-web-tools-test /app

# ------------- server ----------------
FROM scratch AS app

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app /app

ENTRYPOINT [ "/app" ]
