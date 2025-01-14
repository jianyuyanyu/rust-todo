# Build stage
FROM rust:1.83-slim-bullseye as builder

WORKDIR /usr/src/app
COPY . .
RUN cargo --version
# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Install OpenSSL and ca-certificates
RUN apt-get update && \
    apt-get install -y pkg-config openssl ca-certificates libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

# Copy the binary from builder
COPY --from=builder /usr/src/app/target/release/rust-todo .

# Expose the port the app runs on
EXPOSE 3001

# Command to run the application
CMD ["rust-todo"]
