# Use the official Rust image as a parent image
FROM rust:latest

# Install libclang and other dependencies required for librocksdb-sys
RUN apt-get update && apt-get install -y clang libclang-dev

# Set the working directory in the Docker image
WORKDIR /usr/src/myapp

# Copy the current directory contents into the container at /usr/src/myapp
COPY . .

# Update dependencies in the Cargo.lock file
RUN cargo update

# Build the project using the workspace's configuration
# This assumes you want to build the `node` executable
# Adjust the path if you want to build a different member
RUN cargo build --release --workspace

# Use ENTRYPOINT to specify the executable
ENTRYPOINT ["./target/release/node"]

# Set the path to the built executable; adjust this according to the actual executable you want to run
# This example assumes the `node` member produces an executable named `node`
# You might need to adjust the executable name based on the member you're building
CMD []



