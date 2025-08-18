# API Module

A REST API server for managing messages, built with Rust and Axum.

## Overview

This module provides a JSON API for creating and retrieving messages stored in a MySQL database. It uses the shared `core` library for database operations and configuration management.

## Features

- **Create Messages**: POST endpoint to save new messages
- **List Messages**: GET endpoint to retrieve all stored messages  
- **Health Check**: Simple endpoint to verify API status
- **CORS Support**: Permissive CORS configuration for cross-origin requests
- **Error Handling**: Proper HTTP status codes and error responses

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check endpoint |
| `POST` | `/messages` | Create a new message |
| `GET` | `/messages` | Retrieve all messages |

## Dependencies

- **axum**: Web application framework
- **tokio**: Async runtime
- **sqlx**: Database toolkit with MySQL support
- **tower-http**: HTTP middleware (CORS)
- **anyhow**: Error handling

## Configuration

Uses environment variables via the shared `core::Config`:
- Database connection settings
- Server host and port
- Connection pool configuration

## Running

```bash
./message-api
```

The server will start and display:
- Configuration details
- Server address
- Available endpoints