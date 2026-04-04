FROM rust:1.75-alpine AS builder
WORKDIR /app
COPY rust/ .
RUN cargo build --release -p kcode-cli

FROM alpine:3.19
RUN apk --no-cache add ca-certificates
WORKDIR /root/
COPY --from=builder /app/target/release/kcode .

# Expose Webhook port
EXPOSE 3000

ENTRYPOINT ["./kcode"]
CMD ["bridge"]
