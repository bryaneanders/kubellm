# WebApp Module

A web application for managing messages with a user-friendly interface, built with Rust and Axum.

## Overview

This module provides a full-stack web application that combines a backend API with a frontend interface for creating and managing messages. It includes both server-side rendering and API endpoints.

## Features

- **Web Interface**: Clean, responsive UI for message creation
- **Message API**: RESTful endpoint for message operations
- **Static File Serving**: Serves CSS, JS, and other static assets
- **Health Monitoring**: Built-in health check endpoint
- **Real-time Feedback**: Loading states and success/error messages

## Routes

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | Serve the main web interface |
| `GET` | `/health` | Health check endpoint |
| `POST` | `/api/messages` | Create a new message via API |
| `GET` | `/static/*` | Serve static assets |

## Frontend Features

- **Modern Design**: Gradient background with card-based layout
- **Form Validation**: Client-side validation with error feedback
- **Loading States**: Visual feedback during API requests
- **Responsive Design**: Works on desktop and mobile devices
- **Accessibility**: Semantic HTML with proper labels

## Dependencies

- **axum**: Web application framework
- **tokio**: Async runtime
- **sqlx**: Database toolkit with MySQL support
- **tower-http**: HTTP middleware (CORS, static file serving)
- **anyhow**: Error handling
- **serde_json**: JSON serialization

## File Structure

```
webapp/
├── src/
│   └── main.rs          # Main server application
├── static/
│   └── index.html       # Frontend interface
└── Cargo.toml          # Dependencies and configuration
```

## Configuration

Uses environment variables via the shared `core::Config`:
- Database connection settings
- Server host and port
- Connection pool configuration

## Running

```bash
./webapp
```

The web application will start and display:
- Configuration details
- Server address with clickable URL
- Available endpoints

Visit the displayed URL in your browser to access the web interface.