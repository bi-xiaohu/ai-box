# AI-Box — 个人 AI 集成桌面应用

## 项目概述

AI-Box 是一个基于 Tauri 的本地桌面应用，集成多种 AI 功能于一体。MVP 阶段聚焦「多模型 AI 对话」和「RAG 个人知识库」两大核心功能，后续逐步扩展图片生成、语音、OCR、代码助手等能力。

## 技术栈

| 层面 | 选型 |
|------|------|
| 桌面框架 | Tauri v2 |
| 后端语言 | Rust |
| 前端框架 | React + TypeScript (Vite) |
| 数据存储 | SQLite (rusqlite) |
| 向量搜索 | 内嵌方案 (usearch / hnsw_rs 或 sqlite-vss) |
| 样式方案 | Tailwind CSS v4 |

## 架构设计

```
┌──────────────────────────────────────────────┐
│              React + TypeScript               │
│          (对话界面 / 知识库管理 / 设置)          │
├──────────────────────────────────────────────┤
│              Tauri IPC Bridge                 │
├──────────────────────────────────────────────┤
│               Rust Backend                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ LLM 网关  │ │ RAG 引擎  │ │  文档处理器   │  │
│  │          │ │          │ │              │  │
│  │ OpenAI   │ │ Embedding│ │  PDF 解析     │  │
│  │ Claude   │ │ 向量存储  │ │  文本分块     │  │
│  │ Ollama   │ │ 检索排序  │ │  (未来: OCR)  │  │
│  └──────────┘ └──────────┘ └──────────────┘  │
│  ┌──────────────────────────────────────────┐ │
│  │            SQLite + 向量索引              │ │
│  │  对话历史 | 知识库元数据 | 向量 embeddings │ │
│  └──────────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
```

## MVP 已实现功能 (Phase 1) ✅

### 1. 多模型 AI 对话
- ✅ 支持接入 OpenAI (GPT-4o)、Anthropic (Claude)、本地 Ollama 模型
- ✅ 统一的对话界面，可切换模型
- ✅ 对话历史持久化 (SQLite)
- ✅ 支持 Markdown 渲染（react-markdown + GFM）
- ✅ 流式输出 (SSE streaming)

### 2. RAG 个人知识库
- ✅ 文档上传与管理 (支持 txt, md, pdf)
- ✅ 文档自动分块 (512 字符 + 64 字符重叠)
- ✅ 文本 Embedding 生成 (OpenAI text-embedding-3-small)
- ✅ 向量存储 (SQLite BLOB) 与余弦相似度检索
- ✅ 知识库管理 UI（上传/删除/浏览文档）

### 3. 基础设施
- ✅ API Key 管理（存储 + 掩码显示）
- ✅ 全局设置（Base URL、Ollama Host、默认模型）
- ✅ 应用配置持久化 (SQLite settings 表)
- ✅ Copilot Instructions 配置

## 未来计划 (Phase 2+)

### P0 — 核心完善

| 功能 | 说明 | 涉及模块 |
|------|------|---------|
| RAG 对话注入 | 聊天时自动检索知识库相关 chunks，注入 system prompt，实现"基于文档的问答" | `commands/chat.rs`, `ChatView.tsx` |
| API Key 加密存储 | 当前明文存 SQLite，改用 `tauri-plugin-stronghold` 或系统 Keyring | `commands/settings.rs` |

### P1 — 体验优化

| 功能 | 说明 | 涉及模块 |
|------|------|---------|
| Ollama 模型自动发现 | 调用 `GET /api/tags` 动态获取已安装的本地模型列表 | `commands/settings.rs` |
| 对话标题自动生成 | 首次消息后调用 LLM 自动生成对话标题 | `commands/chat.rs` |
| 代码语法高亮 | 集成 highlight.js 或 shiki，Markdown 代码块高亮 | `ChatView.tsx` |
| 错误提示优化 | 统一 Toast 通知，替代当前 console.error 静默处理 | 前端全局 |
| 深色/浅色主题切换 | 读取 settings 中 theme 配置，支持手动切换 | 前端全局 |

### P2 — 功能扩展

| 功能 | 说明 | 涉及模块 |
|------|------|---------|
| 图片生成 | 接入 DALL-E / Stable Diffusion API，独立画图面板 | 新增 `llm/image.rs` + 前端组件 |
| 语音交互 | Whisper 语音转文字 + edge-tts 文字转语音 | 新增 `audio/` 模块 |
| OCR 文档识别 | 扩展 doc_processor 支持图片/扫描 PDF | `doc_processor.rs` |
| 对话导出 | 导出为 Markdown / PDF 文件 | 新增导出命令 + UI |
| 多模型对比 | 同一问题发给多个模型，并排对比回答 | `ChatView.tsx` 扩展 |

### P3 — 架构演进

| 功能 | 说明 | 涉及模块 |
|------|------|---------|
| 插件系统 | 定义插件 API，支持第三方扩展新的 AI 功能 | 新增 `plugins/` 模块 |
| 多窗口/标签页 | 同时打开多个对话窗口 | Tauri 多窗口 + 前端路由 |
| 向量数据库升级 | 文档量大时从暴力搜索迁移到 HNSW 索引 | `embedding.rs` |
| 国际化 (i18n) | 支持中英文界面切换 | 前端全局 |

## 关键 Rust Crates

- `tauri` — 桌面应用框架
- `rusqlite` — SQLite 绑定 (bundled)
- `reqwest` — HTTP 客户端 (json + stream)
- `serde` / `serde_json` — 序列化
- `thiserror` — 错误处理
- `uuid` — ID 生成
- `pdf-extract` — PDF 解析
- `futures` — 异步流处理

## 备注

- 优先保证核心体验流畅，功能可以逐步迭代
- API Key 加密存储是 P0 优先级安全问题
- Ollama 本地模型可实现完全离线使用

