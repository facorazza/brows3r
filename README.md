# BrowS3r

A high-performance S3 file browser and manager built with Rust, Axum, and Askama templates. This is a complete rewrite of the Django-based BrowS3r application.

## Installation

1. Clone the repository:

```bash
git clone https://github.com/yourusername/brows3r.git
cd brows3r
```

1. Copy the example environment file:

```bash
cp .env.example .env
```

1. Configure your environment variables in `.env`:

```env
DATABASE_URL=postgres://username:password@localhost:5432/brows3r
SECRET_KEY=your-secret-key-change-in-production

S3_URL=http://localhost:9000
S3_REGION=us-east-1
S3_BUCKET=mybucket
S3_ACCESS_KEY_ID=minioadmin
S3_ACCESS_KEY_SECRET=minioadmin
```

1. Build and run:

```bash
cargo build --release
cargo run --release
```

The application will be available at `http://localhost:8000`.

### Building with LDAP Support

To enable LDAP authentication, build with the `ldap` feature flag:

```bash
cargo build --release --features ldap
```

Configure LDAP settings in your `.env`:

```env
AUTH_LDAP_SERVER_URI=ldap://localhost:389
AUTH_LDAP_BIND_DN=cn=admin,dc=example,dc=com
AUTH_LDAP_BIND_PASSWORD=password
AUTH_LDAP_USER_DN_TEMPLATE=uid=%(user)s,ou=users,dc=example,dc=com
AUTH_LDAP_GROUP_SEARCH=ou=groups,dc=example,dc=com
```

## Development

### Running Tests

```bash
# Run tests
cargo test

# Run tests with LDAP support
cargo test --features ldap

# Run with logging
RUST_LOG=debug cargo test
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy

# Check without building
cargo check
```

## Docker Deployment

### Build the Docker image

```bash
docker build -f Dockerfile.rust -t brows3r:latest .
```

### Run with Docker

```bash
docker run -d \
  -p 8000:8000 \
  -e DATABASE_URL=postgres://user:pass@postgres:5432/brows3r \
  -e SECRET_KEY=your-secret-key \
  -e S3_URL=http://your-s3:9000 \
  -e S3_REGION=us-east-1 \
  -e S3_BUCKET=mybucket \
  -e S3_ACCESS_KEY_ID=access_key \
  -e S3_ACCESS_KEY_SECRET=secret_key \
  brows3r:latest
```

## API Routes

### Browser Routes (Protected)

- `GET /` - List root directory
- `GET /{path}` - List directory contents
- `POST /create-directory/` - Create new directory
- `DELETE /delete/{path}` - Delete file or directory
- `GET /download/{path}` - Download file
- `POST /upload/` - Upload file (max 100MB)

### User Routes

- `GET /users/login/` - Login form
- `POST /users/login/` - Process login
- `GET /users/logout/` - Logout
- `GET /users/` - List users (protected)

### Health & Metrics Routes

- `GET /health` - Liveness probe
- `GET /ready` - Readiness probe (checks database)
- `GET /metrics` - Prometheus metrics

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | `postgres://localhost/brows3r` | PostgreSQL connection string |
| `SECRET_KEY` | Yes | - | Secret key for session signing (min 32 chars) |
| `S3_URL` | Yes | - | S3 endpoint URL |
| `S3_REGION` | Yes | - | S3 region |
| `S3_BUCKET` | Yes | - | S3 bucket name |
| `S3_ACCESS_KEY_ID` | Yes | - | S3 access key ID |
| `S3_ACCESS_KEY_SECRET` | Yes | - | S3 secret access key |
| `AUTH_LDAP_SERVER_URI` | No* | - | LDAP server URI |
| `AUTH_LDAP_BIND_DN` | No* | - | LDAP bind DN |
| `AUTH_LDAP_BIND_PASSWORD` | No* | - | LDAP bind password |
| `AUTH_LDAP_USER_DN_TEMPLATE` | No* | - | LDAP user DN template |
| `AUTH_LDAP_GROUP_SEARCH` | No* | - | LDAP group search base |

*Required when using the `ldap` feature

```
