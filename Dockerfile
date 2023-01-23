FROM rust:latest AS build

WORKDIR /muse

RUN ["apt-get", "update"]
RUN ["apt-get", "-y", "upgrade"]
RUN ["apt-get", "install", "-y", "cmake"]

RUN ["cargo", "init", "--bin"]
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN ["cargo", "build", "--release"]

RUN ["rm", "-rf", "src"]
COPY ./src ./src
RUN ["cargo", "build", "--release"]


FROM debian:11-slim
COPY --from=build /muse/target/release/muse .
CMD ["./muse"]
