FROM rust:1-slim-bookworm AS builder

WORKDIR /app

COPY . .

RUN cargo build --package mooplas_signalling_server --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install --yes --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --uid 10001 mooplas

WORKDIR /home/mooplas

COPY --from=builder /app/target/release/mooplas_signalling_server /usr/local/bin/mooplas_signalling_server

USER mooplas

EXPOSE 3536

ENTRYPOINT ["/usr/local/bin/mooplas_signalling_server"]
CMD ["--port", "3536"]
