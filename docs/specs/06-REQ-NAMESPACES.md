# 06-REQ-NAMESPACES: Directory Namespaces and Attention Isolation

> [!NOTE] **Architectural Rationale**
> For the research findings and theoretical justification of the Inter-Document Attention Firewall, see [ADR-007: Directory-Based Namespaces and Inter-Document Attention Isolation](../adrs/007-directory-namespaces-inter-document-firewall.md).

## 1. Overview: The Inter-Document Attention Firewall

In multi-document repositories, mixed document types (API specs, architectural rules, runbooks) frequently share identical technical vocabularies. Unpartitioned retrieval causes **Cross-Domain Semantic Bleed**, allowing descriptive specifications to crowd out normative architectural constraints in edge LLM context windows.

Namespaces establish an **Inter-Document Attention Firewall**, ensuring that pre-commit audit gates and code generation agents can deterministically isolate their retrieval corpus.

## 2. Configuration Contract

Namespaces MUST be defined as directory mappings in `.intent/config.toml`:

```toml
[namespaces]
adrs = "docs/adrs"
specs = "docs/specs"
ops  = "runbooks"
```

- **Keys** define the logical namespace identifier (e.g. `"adrs"`, `"specs"`).
- **Values** define the relative or absolute directory path to scan.
- No special frontmatter tags or proprietary formatting are required on individual Markdown files.

## 3. Storage & Schema Contract

The engine MUST maintain a **single unified LanceDB table**, adding a dedicated `namespace: Utf8` column to the Apache Arrow schema:

| Column Name | Arrow Data Type | Nullable | Content |
| :--- | :--- | :---: | :--- |
| `namespace` | `Utf8` | No | Logical namespace tag (e.g. `"adrs"`, `"specs"`). |

Every chunk indexed from a configured directory path inherits its mapped namespace string.

## 4. Query & Filter Pushdown

### 4.1 Scoped Search

When a query specifies a namespace (via CLI `--namespace <name>` or MCP parameter `"namespace": "<name>"`):

- The engine MUST push down an exact SQL equality predicate into the LanceDB query builder:

  ```rust
  let escaped_ns = namespace.replace('\'', "''");
  query.filter(format!("namespace = '{}'", escaped_ns))
  ```

- **Guaranteed Invariant**: Chunks from other namespaces MUST be strictly excluded from the result set regardless of semantic vector similarity.

### 4.2 Global Search

When the namespace parameter is omitted:

- The engine executes a single hybrid search across all namespaces simultaneously.
- Scores and RRF ranks are unified across the entire project knowledge base in a single pass (~2ms).
