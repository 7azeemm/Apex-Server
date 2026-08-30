# Apex Server

**The Rust API backend for Apex's in-game AI chat experience.**

Apex Server sits between the [Apex client](https://github.com/7azeemm/Apex) and the [Python AI service](https://github.com/7azeemm/Apex-AI-Server). It handles player authentication, stored conversations, usage accounting, and streaming responses to the client.

## Current capabilities

- Player authentication using Minecraft profile keys and signature checks.
- Bearer-token sessions and request rate limiting.
- PostgreSQL-backed user records and chat history through SQLx.
- Conversation retrieval and deletion.
- Streaming chat completions through the separate AI service.
- Plan-specific token budgets, context limits, daily resets, and plan-expiration workers.
- Structured logging and Discord webhook notifications.

These describe the implementation in this repository, not a guarantee of production readiness or hosted-service availability.

## Repository layout

| Directory | Purpose |
| --- | --- |
| [core](core) | Main Axum API, authentication, database access, chats, and background workers |
| [common](common) | Shared HTTP, logging, and utility code |
| [skyblock_tools](skyblock_tools) | Experimental SkyBlock data, pricing, wiki, and player tools |
| [monitoring](monitoring) | Docker Compose configuration for Prometheus, Grafana, Loki, and Promtail |

The active Cargo workspace includes **`core` and `common`**. The `skyblock_tools` member is commented out in [Cargo.toml](Cargo.toml), so it is not built or started by the normal workspace commands.

## How the services fit together

1. The Java client authenticates with this server and receives a session token.
2. Authenticated API requests load conversations or submit a new chat message.
3. The backend constructs the allowed conversation context and calls the Python AI service.
4. The response is streamed back to the client; conversation data and usage are updated in PostgreSQL.

The AI service address is currently `http://127.0.0.1:9000`, configured in [core/src/constants.rs](core/src/constants.rs). The public-facing API binds to `0.0.0.0:3000`.

## Development prerequisites

- A recent stable Rust toolchain supporting the Rust 2024 edition.
- PostgreSQL with the application schema already provisioned.
- The Python AI service running separately for chat and title generation.
- Access to the Minecraft services used during authentication.
- Development credentials supplied locally, never committed.

> [!IMPORTANT]
> This repository does not include database migrations or a complete schema bootstrap. Its `sqlx::query!` calls require a compatible database at build time unless prepared SQLx offline metadata is supplied. An empty PostgreSQL database is not enough to build and run the service.

### Configuration

| Setting | Use |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection string; needed for SQLx compilation and at runtime |
| `DISCORD_WEBHOOK_URL` | Webhook used by notification and error-logging paths |
| `AI_SERVER_IP` in `core/src/constants.rs` | AI service URL; currently a source constant, not an environment variable |
| `PLAN_CONFIGS` in `core/src/constants.rs` | Server-side plan limits and behavior |

Set your own values in the environment or an untracked local `.env` file. Use a development database and webhook when testing.

### Build and start

After supplying the matching database schema and configuration:

```bash
git clone https://github.com/7azeemm/Apex-Server.git
cd Apex-Server
cargo check --locked --workspace
cargo run --locked -p core
```

Start the [AI service](https://github.com/7azeemm/Apex-AI-Server) separately. Running the Rust backend alone does not start model inference.

## Main API routes

| Method | Route | Purpose |
| --- | --- | --- |
| `POST` | `/auth` | Validate player credentials and establish a session |
| `POST` | `/interest` | Submit interest in plans |
| `GET` | `/api/chats` | List the authenticated player's conversations |
| `GET` | `/api/chat/{id}` | Retrieve a conversation |
| `DELETE` | `/api/chat/{id}` | Delete a conversation |
| `POST` | `/api/chat/completions` | Request a streamed AI response |

The `/api` routes require a bearer token obtained through the authentication flow. Consult [core/src/api](core/src/api) for payloads and response handling; arbitrary usernames are not a substitute for authenticated client credentials.

## Development status

The backend is part of a multi-service application and still depends on deployment-specific setup. SkyBlock tooling is experimental and outside the active workspace. Monitoring configuration is provided separately and needs its own deployment review.

For self-hosting, review HTTPS termination, database privileges, credential storage, logging, and network exposure before making the API accessible. Authentication and rate limiting in the source should not be treated as a security audit.

## Related repositories

- [Apex](https://github.com/7azeemm/Apex) — Java/Fabric client and chat UI.
- [Apex AI Server](https://github.com/7azeemm/Apex-AI-Server) — Python model integration and response streaming.

## Contributing

Useful improvements include reproducible setup documentation, schema migrations, focused API fixes, and integration tests for the client/backend/AI contract. Do not include credentials, personal conversations, or production database dumps in issues.
