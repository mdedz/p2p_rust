FROM clux/muslrust:stable as builder
WORKDIR /usr/src/tcp_rust

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src
COPY src ./src
COPY . .

RUN rustup target add x86_64-unknown-linux-musl \
 && cargo build --release --target x86_64-unknown-linux-musl

RUN mkdir -p /data

FROM scratch

COPY --from=builder /usr/src/tcp_rust/target/x86_64-unknown-linux-musl/release/tcp_rust /tcp_rust
COPY --from=builder /data /data

WORKDIR /data
EXPOSE 4001 4101

ENTRYPOINT ["/tcp_rust"]
CMD ["--port","4001","--tls"]
