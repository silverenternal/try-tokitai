# Tokitai Experiment Environment
# Reproducible environment for running 30-day experiments
#
# Usage:
#   # Build
#   docker build -t tokitai-experiment:latest .
#
#   # Run 1-day pilot experiment
#   docker run --rm -v $(pwd)/experiments:/app/experiments \
#     -e OPENAI_API_KEY=your_key \
#     tokitai-experiment:latest \
#     cargo run --release -- experiment run --group Ours-Full --days 1
#
#   # Run all groups (30 days)
#   docker run --rm -v $(pwd)/experiments:/app/experiments \
#     -e OPENAI_API_KEY=your_key \
#     -e ANTHROPIC_API_KEY=your_key \
#     tokitai-experiment:latest \
#     cargo run --release -- experiment run --all-groups

FROM rust:1.75-slim-bookworm as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy project files
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY benches/ ./benches/
COPY tests/ ./tests/
COPY experiments/tasks/ ./experiments/tasks/

# Build in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    python3 \
    python3-pip \
    python3-matplotlib \
    python3-pandas \
    python3-scipy \
    python3-seaborn \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies for analysis
RUN pip3 install --no-cache-dir \
    numpy \
    scipy \
    pandas \
    matplotlib \
    seaborn \
    statsmodels

# Create non-root user for security
RUN useradd -m -u 1000 tokitai

# Set working directory
WORKDIR /app

# Copy built binary from builder
COPY --from=builder /app/target/release/ai-assistant /usr/local/bin/

# Copy experiment scripts and tasks
COPY experiments/ ./experiments/
COPY --chown=tokitai:tokitai /app/experiments /app/experiments

# Create directories for logs and data
RUN mkdir -p /app/experiments/logs \
    /app/experiments/analysis/visualizations \
    /app/.tokitai/evolution \
    && chown -R tokitai:tokitai /app

# Switch to non-root user
USER tokitai

# Set environment variables
ENV RUST_LOG=info
ENV PYTHONUNBUFFERED=1

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep ai-assistant || exit 1

# Default command (shows help)
ENTRYPOINT ["ai-assistant"]
CMD ["--help"]
