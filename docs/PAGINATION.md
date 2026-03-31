# Pagination Guide

## Overview

Many Datadog API endpoints return paginated responses. Pup exposes pagination controls as CLI flags so you can navigate through result pages manually. In agent mode, pup also includes pagination metadata in the response envelope, allowing AI agents to detect when more pages are available and construct follow-up requests.

Pup does not auto-paginate (fetch all pages in a loop). Each invocation fetches a single page. To retrieve all results, call the command repeatedly, passing the cursor or offset from the previous response.

## Pagination Patterns

The Datadog API uses three pagination patterns. Pup normalizes these into consistent CLI flags.

### Cursor-Based Pagination

Most V2 API endpoints use cursor-based pagination. The API returns an opaque cursor string in the response metadata. To fetch the next page, pass that cursor back with `--cursor`.

```bash
# First page
pup logs search --query="status:error" --from=1h --limit 10

# Next page (use the cursor value from the previous response)
pup logs search --query="status:error" --from=1h --limit 10 --cursor "eyJhZnRlciI6..."
```

When there are no more pages, the cursor will be empty or absent.

### Offset-Based Pagination

Some endpoints use numeric offset pagination. The API returns a `next_offset` value. To fetch the next page, pass that value with `--page-offset`.

```bash
# First page
pup incidents list --limit 10

# Next page
pup incidents list --limit 10 --page-offset 10
```

When there are no more pages, `next_offset` will be zero or absent.

### Page-Number Pagination

A few endpoints use page-number pagination with `--page-size` and `--page-number`.

```bash
# First page (page 0)
pup service-catalog list --page-size 10

# Second page
pup service-catalog list --page-size 10 --page-number 1
```

## Agent Mode Pagination Metadata

When running in agent mode (`--agent`), pup wraps API responses in an envelope that includes a `pagination` object when pagination metadata is present:

```json
{
  "status": "success",
  "data": [ ... ],
  "pagination": {
    "cursor": "eyJhZnRlciI6...",
    "has_next_page": true,
    "type": "cursor"
  }
}
```

The pagination object may include:

| Field | Description |
|---------|-------------|
| `cursor` | Opaque cursor string for the next page (cursor-based) |
| `next_offset` | Numeric offset for the next page (offset-based) |
| `page` | Current page number (page-number-based) |
| `page_count` | Total number of pages |
| `per_page` | Items per page |
| `total_count` | Total number of items |
| `has_next_page` | Whether more pages are available |
| `type` | Pagination type: `cursor`, `offset`, or `page_number` |

Only relevant fields are included. For example, cursor-based responses include `cursor` and `has_next_page` but not `next_offset`.

## Commands by Pagination Style

### Cursor-Based Commands (`--cursor`)

| Command | Notes |
|---------|-------|
| `audit-logs list` | |
| `audit-logs search` | |
| `cicd events search` | |
| `cicd pipelines list` | |
| `cicd tests list` | |
| `cicd tests search` | |
| `containers list` | |
| `containers images list` | |
| `events search` | |
| `logs search` | Also covers `logs list` and `logs query` aliases |
| `logs list` | Alias for `logs search` |
| `logs query` | Alias for `logs search` |
| `rum events` | |
| `rum sessions list` | |
| `rum sessions search` | |
| `seats users list` | |
| `security findings search` | |
| `security signals list` | |
| `traces search` | |

### Offset-Based Commands (`--page-offset`)

| Command | Notes |
|---------|-------|
| `incidents list` | |
| `reference-tables list` | |

### Page-Number Commands (`--page-size`, `--page-number`)

| Command | Notes |
|---------|-------|
| `obs-pipelines list` | Uses `--limit` for page size, `--page-number` for 0-based page index |
| `service-catalog list` | Both args optional; the API returns no pagination metadata, so agent mode does not emit a `pagination` envelope |

### Commands with Pre-Existing Pagination

These commands already had pagination flags before the unified pagination work. Their existing flags are unchanged.

| Command | Flags |
|---------|-------|
| `app-keys list` | `--page-size`, `--page-number` |
| `cases search` | `--page-size`, `--page-number` |
| `cicd flaky-tests search` | `--cursor`, `--limit` |
| `investigations list` | `--page-limit`, `--page-offset` |
| `llm-obs spans search` | `--cursor`, `--limit` |
| `monitors search` | `--page`, `--per-page` |
| `on-call memberships list` | `--page-size`, `--page-number` |
| `synthetics tests list` | `--page-size`, `--page-number` |
| `synthetics tests search` | `--count`, `--start` |
| `workflows instances list` | `--limit`, `--page` |

## Exempt Commands

The following types of commands do not support pagination:

- **Single-object commands** (e.g., `monitors get`, `dashboards get`) return one item, not a collection
- **Full-collection endpoints** (e.g., `dashboards list`, `slos list`) return all items in a single response
- **V1 bare-array endpoints** (e.g., `monitors list`) return a JSON array with no pagination metadata
- **Non-collection commands** (e.g., `monitors create`, `logs aggregate`) perform actions rather than listing

## Examples

### Paginate through all logs

```bash
# Get first page
pup logs search --query="service:web" --from=1h --limit 50 --output json

# Extract cursor from response, then fetch next page
pup logs search --query="service:web" --from=1h --limit 50 --cursor "eyJhZnRlci..." --output json

# Repeat until cursor is absent or empty
```

### Paginate through incidents with offset

```bash
# Page 1
pup incidents list --limit 20

# Page 2
pup incidents list --limit 20 --page-offset 20

# Page 3
pup incidents list --limit 20 --page-offset 40
```

### Agent mode: detect and follow pagination

```bash
# Agent mode wraps response with pagination metadata
pup --agent logs search --query="*" --from=1h --limit 5

# Response includes:
# "pagination": { "cursor": "...", "has_next_page": true, "type": "cursor" }
# Use the cursor value in the next request
```
