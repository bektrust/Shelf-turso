# Сборка идёт на Linux x86_64 — той же архитектуре, что у Heroku.
# На GitHub-раннере это нативно: ни qemu, ни Rosetta не нужны.
FROM --platform=linux/amd64 heroku/heroku:24 AS build

# Всё, без чего не собираются зависимости:
#   libsql-ffi         -> cmake, clang/libclang (bindgen), cc (build-essential)
#   lettre/native-tls  -> OpenSSL (libssl-dev, pkg-config)
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        cmake \
        clang \
        libclang-dev \
        pkg-config \
        libssl-dev \
        curl \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable

ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /app

# Сначала собираем только зависимости. Этот слой пересобирается лишь при
# изменении Cargo.toml или Cargo.lock, а не на каждую правку в src.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
# COPY сохраняет mtime файла, а он может быть старше заглушки: без touch cargo
# решил бы, что пересобирать нечего, и в образ уехал бы пустой бинарник.
RUN touch src/main.rs && cargo build --release

# Стадия для локальной выгрузки одного бинарника:
#   docker buildx build --target export --output type=local,dest=out .
FROM scratch AS export
COPY --from=build /app/target/release/TopcoatTurso /

# Последняя стадия — то, что получается при обычном `docker build`.
FROM --platform=linux/amd64 heroku/heroku:24 AS runtime

# Бинарник динамически линкуется с OpenSSL 3. В минимальном образе библиотеки
# может не быть, и приложение упало бы на старте с "libssl.so.3: cannot open".
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3t64 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/TopcoatTurso /usr/local/bin/app

# PORT Heroku подставляет сам, HOST нужен, чтобы слушать не только localhost.
ENV HOST=0.0.0.0

CMD ["app"]
