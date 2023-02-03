FROM rust:latest AS build
WORKDIR /muse

RUN apt-get update -y && \
	apt-get install -y cmake

RUN cargo init
COPY ./Cargo.lock ./Cargo.toml ./
RUN cargo build --release && rm -rf src target/release/muse

COPY ./src ./src
RUN cargo build --release


FROM debian:11-slim
WORKDIR /muse

RUN apt-get update -y && \
	apt-get install -y curl ffmpeg python3 && \
	curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp && \
	chmod a+rx /usr/local/bin/yt-dlp

RUN groupadd -r muse && useradd --no-log-init -r -g muse muse
USER muse
COPY --from=build /muse/target/release/muse .
CMD ["./muse"]
