# see: https://www.docker.com/blog/simplify-your-deployments-using-the-rust-official-image/
FROM rust:alpine3.18 as builder
WORKDIR /usr/src/mantra
COPY ./src ./src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
# see: https://github.com/rust-lang/rust/issues/40174
RUN apk add --no-cache musl-dev
RUN cargo install --path .
FROM alpine:3.18
COPY --from=builder /usr/local/cargo/bin/mantra /usr/local/bin/mantra

RUN apk add --no-cache git

CMD ["mantra"]
