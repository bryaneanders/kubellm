# API Module

A REST API server for managing prompts, built with Rust and Axum.

## Overview

This module provides a JSON API for creating and retrieving prompts stored in a MySQL database. It uses the shared `core` library for database operations and configuration management.

## Features

- **Create Messages**: POST endpoint to save new prompts
- **List Messages**: GET endpoint to retrieve all stored prompts  
- **Health Check**: Simple endpoint to verify API status
- **CORS Support**: Permissive CORS configuration for cross-origin requests
- **Error Handling**: Proper HTTP status codes and error responses

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check endpoint |
| `POST` | `/prompts` | Create a new prompt |
| `GET` | `/prompts` | Retrieve all prompts |

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
./prompt-api
```

The server will start and display:
- Configuration details
- Server address
- Available endpoints