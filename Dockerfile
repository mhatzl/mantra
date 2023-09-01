# see: https://www.docker.com/blog/simplify-your-deployments-using-the-rust-official-image/
FROM rust:alpine3.18 as builder
WORKDIR /usr/src/mantra
COPY ./mantra .
# see: https://github.com/rust-lang/rust/issues/40174
RUN apk add --no-cache musl-dev
RUN cargo install --path .
FROM alpine:3.18
COPY --from=builder /usr/local/cargo/bin/mantra /usr/local/bin/mantra

# To mount the folder containing requirements of the wiki
RUN mkdir /req_folder
VOLUME /req_folder

# To mount the project folder
RUN mkdir /proj_folder
VOLUME /proj_folder

# Makes it possible to run different commands with one image
ENV MANTRA_CMD=sync
ENV MANTRA_ARGS=""

ENTRYPOINT mantra $MANTRA_CMD $MANTRA_ARGS /req_folder /proj_folder
