# LocalGPT Server

A background daemon that provides the core LocalGPT services over HTTP and via external bridges. This crate implements the HTTP REST API, the embedded Web UI, and integration for bots like Telegram.

## Features

- **HTTP REST API**: Unified interface for LocalGPT reasoning and system management.
- **Embedded Web UI**: Self-contained web application for interacting with the assistant from any browser.
- **Telegram Bot Integration**: Native support for interacting with LocalGPT via Telegram.
- **Secure Bridge IPC**: Facilitates encrypted communication with standalone bridge daemons.
- **Websocket Support**: Real-time interaction and status updates.
