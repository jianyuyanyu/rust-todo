# Rust Todo API

A RESTful API built with Rust for managing todo items and practice records.

## Features

- User authentication with JWT
- Create and manage practice actions
- Track practice records
- PostgreSQL database backend
- Docker support

## Quick Start with Docker

You can quickly run this application using Docker:

```bash
# Pull the image
docker pull ghcr.io/sangmingming/rust-todo:latest

# Run with Docker Compose
wget https://raw.githubusercontent.com/sangmingming/rust-todo/main/docker-compose.yml
docker-compose up -d
```

## Environment Variables

The following environment variables can be configured:

- `POSTGRES_USER` - PostgreSQL username (default: postgres)
- `POSTGRES_PASSWORD` - PostgreSQL password (default: postgres)
- `POSTGRES_DB` - PostgreSQL database name (default: mydata)
- `POSTGRES_HOST` - PostgreSQL host (default: localhost)
- `POSTGRES_PORT` - PostgreSQL port (default: 5432)
- `PORT` - API server port (default: 3001)
- `JWT_SECRET` - Secret key for JWT tokens

## API Endpoints

- POST `/api/register` - Register a new user
- POST `/api/login` - Login and get JWT token
- GET `/api/actions` - List all practice actions
- POST `/api/actions` - Create a new practice action
- GET `/api/actions/:id` - Get a specific action
- POST `/api/actions/:id/finish` - Mark an action as finished
- GET `/api/actions/:id/records` - Get records for an action

## Development

### Prerequisites

- Rust 1.74 or later
- PostgreSQL 14 or later

### Local Development

1. Clone the repository
2. Copy `.env.example` to `.env` and configure
3. Run `cargo run`

