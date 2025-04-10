# Historical Volatility API

This project provides a simple **Axum-based HTTP API** for calculating **historical volatility** of Solana tokens using data fetched from the [Birdeye API](https://birdeye.so/).


## ✨ Features

- Fetches **daily** historical token prices from Birdeye.
- Calculates **average daily volatility** for a specific date range.
- Exposes a **health check** endpoint.
- Structured **JSON responses**.
- Automatic **request/response logging**.
- Simple, focused, and lightweight.

## 📚 Endpoints

### `GET /historicalVolatility`

Calculates historical volatility for a given token and time range.

#### Query Parameters (all **required**):

| Name | Type | Example | Description |
| --- | --- | --- | --- |
| `fromDate` | String | `2024-12-31` | Start date in format `YYYY-MM-DD`. |
| `toDate` | String | `2025-03-31` | End date in format `YYYY-MM-DD`. |
| `tokenAddress` | String | `So11111111111111111111111111111111111111112` | Solana token address to calculate for. |

#### Example Request

```bash
curl "http://localhost:3000/historicalVolatility?fromDate=2024-12-31&toDate=2025-03-31&tokenAddress=So11111111111111111111111111111111111111112"
```

#### Success Response (`200 OK`)

```json
{
  "historicalVolatility": 7.5
}
```

---

### `GET /healthCheck`

Simple endpoint to check if the server is alive.

#### Example Request

```bash
curl "http://localhost:3000/healthCheck"
```

#### Success Response (`200 OK`)

```json
{
  "message": "Server is running."
}
```

---

## ⚠️ Error Responses

All errors return a consistent **JSON** format with appropriate HTTP status codes.

| Status Code | Example Error Response |
| --- | --- |
| `400 Bad Request` | `{ "error": "Bad Request", "message": "Invalid fromDate format." }` |
| `400 Bad Request` | `{ "error": "Bad Request", "message": "Failed to deserialize query string: missing field 'fromDate'" }` |
| `500 Internal Server Error` | `{ "error": "Internal Server Error", "message": "Something bad happened." }` |

---

## ⚙️ Environment Variables

You must configure the following environment variables:

| Name | Example | Required |
| --- | --- | --- |
| `BIRDEYE_API_KEY` | `your-api-key-here` | ✅ |
| `BIRDEYE_BASE_URL` | `https://public-api.birdeye.so/token_price/history` | ✅ |
| `APP_SERVER_PORT` | `3000` | ✅ |

Example `.env` file:

```bash
BIRDEYE_API_KEY=your-api-key-here
BIRDEYE_BASE_URL=https://public-api.birdeye.so/token_price/history
APP_SERVER_PORT=3000
```

---

## 🚀 Running Locally

```bash
cargo run
```

The server will start and listen on `0.0.0.0:${APP_SERVER_PORT}`.

---

## 💡 Logging

The app logs:

- Incoming HTTP requests (method, URI).
- Query parameters extracted.
- Successful and failed responses.
- Errors with full JSON bodies.

All logging is done using the [`tracing`](https://docs.rs/tracing/) ecosystem.

---

## ✅ Example Local Usage

```bash
curl "http://localhost:3000/historicalVolatility?fromDate=2024-12-31&toDate=2025-03-31&tokenAddress=So11111111111111111111111111111111111111112"

# Response:
# { "historicalVolatility": 7.5 }

curl "http://localhost:3000/healthCheck"

# Response:
# { "message": "Server is running." }
```

---

## 📋 Notes

- Dates must be in `YYYY-MM-DD` format.
- `tokenAddress` validity is not verified — Birdeye handles validation.

---



