FROM scratch

COPY target/x86_64-unknown-linux-musl/release/waddle-ws /waddle-ws
EXPOSE 3000
ENTRYPOINT ["/waddle-ws"]
