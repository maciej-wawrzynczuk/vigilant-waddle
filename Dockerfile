FROM docker.io/debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y tini=0.19.0-1+b3 \
        --no-install-recommends && \
    rm -rf /var/lib/apt/lists

COPY target/debug/waddle-ws /waddle-ws
EXPOSE 3000
ENV WADDLE_LISTEN_ADDR="0.0.0.0:3000"
ENV RUST_LOG=debug
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/waddle-ws"]
