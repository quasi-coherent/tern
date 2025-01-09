FROM postgres:16-bookworm

RUN apt update && apt install -y \
    wget \
    unzip \
    make \
    build-essential \
    postgresql-server-dev-16 && \
    rm -rf /var/lib/apt/lists/*

RUN wget https://github.com/pgpartman/pg_partman/archive/v5.2.4.zip -O v5.2.4.zip && \
    unzip v5.2.4.zip && \
    cd pg_partman-5.2.4 && \
    NO_BGW=1 make install
