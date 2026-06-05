FROM rust:latest

RUN apt-get update && apt-get install -y \
    strace \
    gdb \
    procps \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY  . .
RUN cargo build
CMD ["./target/debug/tinyinit", "/bin/sh"]