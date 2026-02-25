# AI-Box — Copilot Instructions

## Build & Run

```bash
npm run tauri dev          # Dev mode (frontend HMR + Rust auto-recompile)
npm run tauri build        # Release build (produces .msi installer)
npm run build              # Frontend only: tsc + vite build
```

No test runner is configured yet. Rust modules have inline `#[cfg(test)]` tests runnable via:

```bash
cd src-tauri && cargo test                    # All tests
cd src-tauri && cargo test doc_processor      # Single module
cd src-tauri && cargo test embedding::tests   # Specific test module
```

## Architecture

This is a **Tauri v2 desktop app** with a Rust backend and React frontend. All communication happens through Tauri's IPC bridge.

### Data Flow

```
React Component → invoke("command", {args}) → #[tauri::command] fn → Database/LLM/etc → Result<T, String>
```

For streaming (chat): the backend emits `"chat-stream"` events via `app.emit()`, and the frontend listens with `listen<ChatStreamEvent>()`.

### LLM Provider Pattern

`llm/mod.rs` defines a `Provider` enum (not a trait) dispatching to OpenAI-compatible or Claude backends. Ollama reuses the OpenAI implementation with a different `base_url`. Model strings use the format `"provider/model-id"` (e.g., `"openai/gpt-4o"`, `"ollama/llama3"`). Provider resolution happens in `commands/chat.rs::resolve_provider()`.

### State Management

- **Backend**: A single `Database` struct wraps `Mutex<Connection>` (SQLite), registered as Tauri managed state. All DB access goes through `db.conn.lock().unwrap()`.
- **Frontend**: Top-level state lives in `App.tsx` and flows down via props. No state management library — just `useState`/`useEffect`.

### Embedding & RAG

Documents are parsed (`doc_processor.rs`), chunked with overlap, and embedded via OpenAI's embedding API. Vectors are stored as BLOBs in SQLite's `chunks` table and searched with brute-force cosine similarity (`embedding.rs`). There is no vector database.

## Conventions

### Rust Backend

- **Tauri commands** return `Result<T, String>`. Convert errors with `.map_err(|e| e.to_string())`.
- **Async commands** must not hold `MutexGuard` across `.await` points — extract data from DB in a sync block, drop the lock, then await.
- **New commands** go in `src-tauri/src/commands/` as a submodule, then register in `lib.rs`'s `generate_handler![]` macro.
- **IDs** are generated with `uuid::Uuid::new_v4().to_string()`.

### React Frontend

- **All Tauri `invoke()` calls** are wrapped in `src/lib/api.ts` with typed return values. Never call `invoke()` directly from components.
- **Components** are functional with TypeScript interfaces for props. Callbacks are named `onAction` (e.g., `onSelect`, `onDelete`).
- **Styling** uses Tailwind CSS v4 utility classes inline. The only CSS file is `App.css` which imports Tailwind.
- **TypeScript is strict** — `noUnusedLocals` and `noUnusedParameters` are enforced.

### Model String Format

Models follow `"provider/model-id"` convention throughout the stack:
- `"openai/gpt-4o"`, `"claude/claude-sonnet-4-20250514"`, `"ollama/llama3"`
- The prefix determines which LLM provider and API key to use.
