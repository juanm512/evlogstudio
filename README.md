# evlogstudio

A log ingest and query server, self-hosted, single binary, configurable via environment variables.

## Environment Variables

The server is configured exclusively through environment variables:

| Variable | Required | Default | Description |
| :--- | :--- | :--- | :--- |
| `PORT` | No | `8080` | Port the server listens on. |
| `HOST` | No | `0.0.0.0` | Network interface the server listens on. |
| `STORAGE_MODE` | No | `local` | Storage mode: `local`, `motherduck`, or `s3`. |
| `DATA_PATH` | No | `/data/logs.duckdb` | Path to the DuckDB file (only for `local` mode). |
| `MOTHERDUCK_TOKEN` | Conditional | — | Required if `STORAGE_MODE=motherduck`. |
| `S3_BUCKET` | Conditional | — | Required if `STORAGE_MODE=s3`. |
| `S3_REGION` | No | `us-east-1` | S3 bucket region. |
| `S3_ENDPOINT` | No | — | Custom endpoint (R2, MinIO, etc.). |
| `S3_ACCESS_KEY_ID` | Conditional | — | Required if `STORAGE_MODE=s3`. |
| `S3_SECRET_ACCESS_KEY` | Conditional | — | Required if `STORAGE_MODE=s3`. |

## Storage Modes

### local
Uses a local DuckDB file for immediate persistence. This is the simplest mode for deployments with persistent disks.

**Example Command:**
```bash
STORAGE_MODE=local DATA_PATH=./logs.duckdb ./evlogstudio
```

### motherduck
Connects to [MotherDuck](https://motherduck.com/) to store logs in the cloud. It allows querying logs from anywhere without managing database infrastructure. Data is synced automatically.

**Example Command:**
```bash
STORAGE_MODE=motherduck MOTHERDUCK_TOKEN=your_token_here ./evlogstudio
```

### s3
Ideal for platforms with ephemeral file systems (like Heroku or Docker without volumes). 
- Logs are written to a local buffer (`/tmp/evlogstudio_buffer.duckdb`).
- Every 60 seconds, the buffer is synced with the S3 bucket (exported to Parquet and uploaded).
- Upon startup, the server downloads the latest state from S3.

**Example Command (using Cloudflare R2):**
```bash
STORAGE_MODE=s3 \
S3_BUCKET=my-logs \
S3_ENDPOINT=https://<account_id>.r2.cloudflarestorage.com \
S3_ACCESS_KEY_ID=xxx \
S3_SECRET_ACCESS_KEY=yyy \
S3_REGION=auto \
./evlogstudio
```

## Deployment

### VPS / bare metal
1. Download the binary for your architecture.
2. Create a systemd service to ensure the server runs in the background.

**Example `/etc/systemd/system/evlogstudio.service`:**
```ini
[Unit]
Description=evlogstudio log server
After=network.target

[Service]
Type=simple
User=root
Environment=PORT=8080
Environment=STORAGE_MODE=local
Environment=DATA_PATH=/var/lib/evlogstudio/logs.duckdb
ExecStart=/usr/local/bin/evlogstudio
Restart=always

[Install]
WantedBy=multi-user.target
```

### Docker
**Local mode (with volume):**
```bash
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/data:/data \
  -e STORAGE_MODE=local \
  -e DATA_PATH=/data/logs.duckdb \
  evlogstudio
```

**S3 mode (no persistent volume):**
```bash
docker run -d \
  -p 8080:8080 \
  -e STORAGE_MODE=s3 \
  -e S3_BUCKET=my-logs \
  -e S3_ACCESS_KEY_ID=xxx \
  -e S3_SECRET_ACCESS_KEY=yyy \
  evlogstudio
```

### Railway
1. Connect your repository.
2. In the dashboard, configure the required environment variables.
3. If using `local` mode, add a **Volume** and point `DATA_PATH` to a path inside that volume. Otherwise, use `s3` mode.

### Render
1. Create a **Web Service**.
2. If using `local` mode, add a **Persistent Disk** at `/data` and configure `DATA_PATH=/data/logs.duckdb`.
3. Configure the environment variables in the "Environment" tab.

### Fly.io
1. Run `fly launch`.
2. Create a volume: `fly volumes create evlog_data --size 1`.
3. Configure `fly.toml` to mount the volume:
```toml
[mounts]
  source = "evlog_data"
  destination = "/data"
```
4. Set sensitive variables: `fly secrets set MOTHERDUCK_TOKEN=...`.

## Initial Setup

When starting the server for the first time (when no users are registered), you will see a banner in the terminal with a setup token:

```text
    ╔══════════════════════════════════════════════════╗
    ║  Initial setup required                          ║
    ║                                                  ║
    ║  Complete setup at:                              ║
    ║  POST /setup?token=xxxxxxxx-xxxx-xxxx-xxxx-xxxx  ║
    ╚══════════════════════════════════════════════════╝
```

To complete the setup, you must create the first administrator user:

```bash
curl -X POST "http://localhost:8080/setup?token=YOUR_BANNER_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"email": "admin@example.com", "password": "a_secure_password"}'
```

> [!IMPORTANT]
> If you lose the token or need to restart the process, the banner will only be shown if the user table is empty. If users already exist, the server will start normally without showing the token.

## API Reference

### Health Check
- **GET** `/health`
- **Auth**: None
- **Response**: `200 OK`
```json
{"status": "ok"}
```

### Initial Setup
- **POST** `/setup`
- **Auth**: Query param `token`
- **Body**: `{"email": "...", "password": "..."}`
- **Response**: `200 OK`
```json
{"message": "admin created"}
```

### Login
- **POST** `/auth/login`
- **Auth**: None
- **Body**: `{"email": "...", "password": "..."}`
- **Response**: `200 OK`
```json
{
  "token": "JWT_TOKEN_HERE",
  "role": "admin"
}
```

### Query Logs
- **GET** `/api/logs`
- **Auth**: JWT (Any role)
- **Query Params**:
  - `source`: Filter by source.
  - `level`: Filter by level (info, error, etc.).
  - `from`: Start ISO-8601 date.
  - `to`: End ISO-8601 date.
  - `search`: Text search in message.
  - `limit`: Number of results (default 50, max 200).
  - `cursor`: ID of the last log for pagination.
- **Response**:
```json
{
  "logs": [...],
  "next_cursor": "LAST_LOG_ID"
}
```

### Poll Logs
- **GET** `/api/logs/poll`
- **Auth**: JWT (Any role)
- **Query Params**:
  - `since_id`: ID of the last log seen (derives the temporal cutoff).
  - `since_timestamp`: ISO-8601 fallback if `since_id` is missing.
- **Response**:
```json
{
  "logs": [...],
  "last_id": "...",
  "last_timestamp": "...",
  "count": 5
}
```

### Schema
- **GET** `/api/schema`
- **Auth**: JWT (Any role)
- **Query Params**: `source` (optional)
- **Response**:
```json
{
  "fields": [
    { "source": "backend-tuerca", "field_path": "user.id", "field_type": "string", "seen_count": 42, "last_seen": "..." }
  ],
  "sources": ["backend-tuerca", "frontend-app"]
}
```

### Source Management
- **GET** `/api/sources`: List all sources (JWT - Any role).
- **POST** `/api/sources`: Create a source (JWT admin). Body: `{"name": "...", "description": "...", "retention_days": 30}`.
- **DELETE** `/api/sources/:id`: Delete a source (JWT admin).

### Token Management
- **GET** `/api/sources/:id/tokens`: List tokens for a source (JWT admin).
- **POST** `/api/sources/:id/tokens`: Create an ingest token (JWT admin). Body: `{"name": "..."}`.
- **DELETE** `/api/tokens/:id`: Revoke a token (JWT admin).

### User Management
- **GET** `/api/users`: List users (JWT admin).
- **POST** `/api/users`: Create a user (JWT admin). Body: `{"email": "...", "password": "...", "role": "viewer"}`.
- **PUT** `/api/users/:id/role`: Change a user's role (JWT admin). Body: `{"role": "admin"}`.
- **DELETE** `/api/users/:id`: Delete a user (JWT admin). You cannot delete yourself.

### Ingest
- **POST** `/ingest`
- **Auth**: Header `Authorization: Bearer <INGEST_TOKEN>`
- **Body**: JSON object or array of objects.
- **Response**: `{"inserted": 10}`.

### Analytics
- **GET** `/api/analytics/volume`
- **Auth**: JWT
- **Query Params**: `interval` (minute, hour, day, week), `from`, `to`.
- **GET** `/api/analytics/errors`
- **Auth**: JWT
- **Query Params**: `from`, `to`.

## evlog Integration

`evlogstudio` is compatible with the `evlog` HTTP drain.

### Nuxt / Nitro — server/plugins/evlog.ts
```typescript
import { createHttpLogDrain } from 'evlog/http'

export default defineNitroPlugin((nitroApp) => {
  nitroApp.hooks.hook('evlog:drain', createHttpLogDrain({
    drain: {
      endpoint: 'https://tu-servidor.com/ingest',
      headers: { Authorization: 'Bearer tu_ingest_token' }
    }
  }))
})
```

### Express / Hono / Any framework
```typescript
import { createHttpLogDrain } from 'evlog/http'

app.use(evlog({ drain: createHttpLogDrain({
  drain: {
    endpoint: 'https://tu-servidor.com/ingest',
    headers: { Authorization: 'Bearer tu_ingest_token' }
  }
})}))
```

## Testing Storage Modes

### Test MotherDuck
1. Create your account at [motherduck.com](https://motherduck.com/).
2. Copy your token from the dashboard.
3. Run: `STORAGE_MODE=motherduck MOTHERDUCK_TOKEN=your_token ./evlogstudio`
4. Send logs:
   ```bash
   curl -X POST http://localhost:8080/ingest \
        -H "Authorization: Bearer YOUR_INGEST_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"msg": "test motherduck"}'
   ```
5. Verify in the MotherDuck dashboard that the `evlogstudio` database has the tables created.

### Test S3 / R2 (Cloudflare)
1. Create an R2 bucket.
2. Create an API Token with `Object Read & Write` permissions.
3. Set the variables and start:
   ```bash
   STORAGE_MODE=s3 \
   S3_BUCKET=my-bucket \
   S3_ENDPOINT=https://<account_id>.r2.cloudflarestorage.com \
   S3_ACCESS_KEY_ID=... \
   S3_SECRET_ACCESS_KEY=... \
   ./evlogstudio
   ```
4. Send logs and wait 60 seconds.
5. Verify in the Cloudflare dashboard that the file `logs/latest.parquet` exists.

### Test AWS S3
1. Create an AWS S3 bucket.
2. Create an IAM user with `s3:GetObject` and `s3:PutObject` policy.
3. Run without `S3_ENDPOINT`:
   ```bash
   STORAGE_MODE=s3 S3_BUCKET=my-aws-bucket ... ./evlogstudio
   ```
