# CLI Module

A command-line interface for managing messages, built with Rust and Clap.

## Overview

This module provides a CLI tool for interacting with the message system. It allows users to initialize the database, create messages, list existing messages, and check database connectivity status.

## Binary Name

The CLI tool is built as `message-cli` (defined in Cargo.toml).

## Commands

### Database Management
- `init-db` - Initialize the database schema

### Message Operations  
- `list` - Display all stored messages with timestamps
- `create -m "message content"` - Create a new message

### System Operations
- `status` - Check database connection and display configuration

## Usage Examples

```bash
# Initialize the database
message-cli init-db

# Create a new message
message-cli create --message "Hello, world!"
message-cli create -m "Another message"

# List all messages
message-cli list

# Check database status
message-cli status

# Get main help message
message-cli --help

# Get help for a specific command
message-cli create --help
```

## Dependencies

- **clap**: Command-line argument parser with derive macros
- **tokio**: Async runtime
- **anyhow**: Error handling
- **core**: Shared library for database operations and configuration

## Configuration

Uses environment variables via the shared `core::Config`:
- Database connection settings
- Server configuration

## Output Format

- **List command**: Shows message ID, creation timestamp, and content
- **Create command**: Displays the ID of the newly created message
- **Status command**: Shows database connection status and URL
- **Init command**: Confirms successful database initialization