# 04-REQ-NAMESPACES-FILTERING

## 1. Overview

The Intent Engine MUST support multiple disparate data sources (ADRs, Runbooks, Blog Posts, API Specs) simultaneously without polluting search results.

## 2. Namespace Isolation

**Requirement:** The engine MUST abandon the hardcoded `"adrs"` table name.
**Implementation details:**

- Every MCP method (`recall`, `index_directory`) requires a `namespace` string parameter.
- The `index_adrs` function MUST be renamed to `sync_namespace` and accept this parameter.
- LanceDB table connections MUST be dynamic: `db.open_table(namespace)`.
- If `db.open_table` fails during indexing because the table does not exist, the engine MUST seamlessly fall back to `db.create_table(namespace, ...)` to initialize it.

**Acceptance Criteria:**

- **Given** a request to index `/runbooks` into namespace `ops`, **When** executed, **Then** LanceDB MUST contain a table named `ops`, and a manifest named `.manifest_ops.json` MUST exist.

## 3. Metadata SQL Pushdown

**Requirement:** The engine MUST leverage LanceDB's DataFusion SQL engine to pre-filter vectors based on the JSON metadata extracted from YAML frontmatter.

**Implementation details:**

- When the `recall` tool receives a `metadata_filter` string (e.g., `metadata.author = 'VJ'`).
- The Engine MUST append this to the LanceDB query builder:

  ```rust
  let mut query_builder = table.search(query_vector).limit(limit);
  if let Some(filter) = metadata_filter {
      query_builder = query_builder.filter(filter);
  }
  let results = query_builder.execute().await?;
  ```

**Acceptance Criteria:**

- **Given** two documents indexed in the `adrs` namespace (one with `status: accepted` and one with `status: rejected`).
- **When** the AI calls `recall` with `metadata_filter: "metadata.status = 'accepted'"`.
- **Then** the results MUST strictly exclude chunks from the rejected document, regardless of semantic similarity.
