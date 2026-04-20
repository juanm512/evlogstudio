# evlogstudio

A log ingest and query server, self-hosted, single binary, configurable via environment variables.

## Installation

### One-line install (Linux / macOS)

```bash
curl -sSL https://raw.githubusercontent.com/juanm512/evlogstudio/main/install.sh \
  -o /tmp/install.sh && sudo bash /tmp/install.sh
```

> Note: downloading before running with sudo avoids pipe permission issues.

### Without sudo (installs to ~/.local/bin)

```bash
curl -sSL https://raw.githubusercontent.com/juanm512/evlogstudio/main/install.sh \
  -o /tmp/install.sh && bash /tmp/install.sh
```

The installer auto-detects permissions and installs to `~/.local/bin` if not running as root.

The installer will:
1. Detect your OS and architecture
2. Download the latest binary from GitHub Releases
3. Ask for basic configuration (port, storage mode)
4. Optionally install a systemd service (Linux only)
5. Print the URL to open in your browser

### Manual install

Download the binary for your platform from
https://github.com/juanm512/evlogstudio/releases/latest

```
Linux amd64:  evlogstudio-linux-amd64
Linux arm64:  evlogstudio-linux-arm64
macOS arm64:  evlogstudio-darwin-arm64
macOS amd64:  evlogstudio-darwin-amd64
```

```bash
chmod +x evlogstudio-linux-amd64
PORT=8080 DATA_PATH=./logs.duckdb ./evlogstudio-linux-amd64
```

### Docker

```bash
docker run -d \
  -p 8080:8080 \
  -v evlog_data:/data \
  ghcr.io/juanm512/evlogstudio:latest
```

### Version check

```bash
evlogstudio --version
```

## Upgrading

Re-run the install script to upgrade to the latest version:

```bash
curl -sSL https://raw.githubusercontent.com/juanm512/evlogstudio/main/install.sh | bash
```

Your data is preserved — the installer only replaces the binary.
If using systemd, the service restarts automatically.

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

## Deployment & Cloud Hosting

### Docker (Recommended)

The easiest way to run `evlogstudio` is using the official image from GitHub Container Registry:

```bash
docker run -d \
  -p 8080:8080 \
  -v evlog_data:/data \
  -e STORAGE_MODE=local \
  -e DATA_PATH=/data/logs.duckdb \
  ghcr.io/juanm512/evlogstudio:latest
```

### Cloud Providers

#### Fly.io 
Follow these steps to deploy using the provided `fly.toml`:
1. **Create the App**: `fly apps create evlogstudio`
2. **Create the Volume**: `fly volumes create evlog_data --region gru --size 1`
3. **Deploy**: `fly deploy`

#### Render
1. Create a **Web Service** -> **Existing Image**.
2. Use: `ghcr.io/juanm512/evlogstudio:latest`.
3. Add a **Persistent Disk** at `/data`.
4. Set `STORAGE_MODE=local` and `DATA_PATH=/data/logs.duckdb`.

#### Railway
1. Create a **New Project** -> **Deploy from Docker image**.
2. Use: `ghcr.io/juanm512/evlogstudio:latest`.
3. Add a **Volume** and mount it at `/data`.
4. Set Environment Variables: `STORAGE_MODE=local`, `DATA_PATH=/data/logs.duckdb`.

---

### Important: Persistence & Costs

When deploying to the cloud, keep these constraints in mind:

| Feature | Fly.io | Render | Railway |
| :--- | :--- | :--- | :--- |
| **Free Tier** | No (Pay-as-you-go) | Yes (Web Service) | Trial only ($5) |
| **Persistence** | Yes (Paid Volume) | **No** (Paid plan only) | Yes (Paid Volume) |
| **Always On** | Yes | No (Sleeps after 15m) | Yes |

> [!WARNING]
> **Data Loss on Render Free**: Render's free tier does NOT support persistent disks. If you deploy to Render Free, your logs will be deleted every time the service restarts or sleeps. For a production logs server, use a paid plan with a Disk or a different provider.

> [!TIP]
> **Truly $0 Hosting?**: If you need a permanent $0 server with persistence, consider **Oracle Cloud Free Tier** (Always Free ARM VMs) or self-hosting on a local device (Raspberry Pi/Old Laptop) using **Cloudflare Tunnels**.

## Initial Setup

When starting the server for the first time (when no users are registered), simply open the server's URL in your browser. You will be automatically redirected to the setup page to create your initial administrator account. 

Once created, you'll be logged in and directed to the logs dashboard.

> [!NOTE]
> The setup page is only accessible if the user table is empty.


## API Reference

### Health Check
- **GET** `/health`
- **Auth**: None
- **Response**: `200 OK`
```json
{"status": "ok", "setup_required": false}
```

### Initial Setup
- **GET** `/setup`: Returns `200 OK` if setup is required, else `404`.
- **POST** `/setup`
- **Auth**: None (only available when no users exist)
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
