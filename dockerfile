# Start with a minimal base image with necessary dependencies
FROM debian:bookworm-slim

# Install libxml2 libraries
RUN apt-get update && \
    apt-get install -y libxml2 libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

# Set environment variables
ENV RUST_LOG=info

# Create a new directory for the application
WORKDIR /app

# Copy the release binary from the build stage
COPY target/release/qualifyrss .

# Expose any ports the application uses
EXPOSE 8080

# Run the binary
CMD ["./qualifyrss"]
