# Build stage
FROM rust:1-bookworm AS build
WORKDIR /work
COPY . .
# Only build svc-gateway to keep layer smaller; relies on workspace Cargo.toml
RUN cargo build -p svc-gateway --release

# Runtime (distroless-ish, small)
FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=build /work/target/release/svc-gateway /app/svc-gateway
USER nonroot:nonroot
EXPOSE 5304 9301
ENV RUST_LOG=info,svc_gateway=info
ENTRYPOINT ["/app/svc-gateway"]
