FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ADD https://github.com/liagha/termgram/releases/download/v0.1.1/termgram-mcp /usr/local/bin/termgram-mcp

RUN chmod +x /usr/local/bin/termgram-mcp

USER nobody

ENTRYPOINT ["/usr/local/bin/termgram-mcp", "--catalog"]