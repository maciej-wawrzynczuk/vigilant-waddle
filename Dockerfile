FROM scratch

COPY target/release/waddle-ws /waddle-ws
EXPOSE 3000
ENTRYPOINT ["/waddle-ws"]
