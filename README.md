# FDK MQA Scoring API

This application provides an API for retrieving scores.

For a broader understanding of the system’s context, refer to
the [architecture documentation](https://github.com/Informasjonsforvaltning/architecture-documentation) wiki. For more
specific context on this application, see the **Metadata Quality** subsystem section.

## Getting Started
These instructions will give you a copy of the project up and running on your local machine for development and testing purposes.

### Prerequisites

Ensure you have the following installed:
- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)

### Running locally

Clone the repository:

```sh
git clone https://github.com/Informasjonsforvaltning/fdk-mqa-scoring-api.git
cd fdk-mqa-scoring-api
```

Build for development:

```sh
cargo build --verbose
```

Start PostgreSQL:

```sh
docker compose up -d
```

Migrate database and start application :

```sh
API_KEY=foo POSTGRES_HOST=localhost POSTGRES_PORT=5432 POSTGRES_USERNAME=postgres POSTGRES_PASSWORD=postgres POSTGRES_DB_NAME=mqa CORS_ORIGIN_PATTERNS="http://localhost:*" cargo r
```

### API Documentation (OpenAPI)

The API documentation is available at ```openapi.yaml```.

### Running tests

```sh
cargo t
```
