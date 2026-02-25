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

## MVP 功能范围 (Phase 1)

### 1. 多模型 AI 对话
- 支持接入 OpenAI (GPT-4o)、Anthropic (Claude)、本地 Ollama 模型
- 统一的对话界面，可切换模型
- 对话历史持久化 (SQLite)
- 支持 Markdown 渲染、代码高亮
- 流式输出 (SSE/streaming)

### 2. RAG 个人知识库
- 文档上传与管理 (支持 txt, md, pdf)
- 文档自动分块 (chunking)
- 文本 Embedding 生成 (调用 OpenAI Embedding API 或本地模型)
- 向量存储与相似度检索
- 对话时可选择关联知识库，自动检索相关内容注入 prompt

### 3. 基础设施
- API Key 管理 (加密存储)
- 全局设置 (主题、默认模型、Ollama 地址等)
- 应用配置持久化

## 后续阶段 (Phase 2+)

- 图片生成 (Stable Diffusion / DALL-E)
- 语音识别 / TTS (Whisper / edge-tts)
- OCR 文档识别
- 代码助手
- 插件系统

## MVP 实施计划

### Todo 列表

1. ✅ **project-init** — 初始化 Tauri v2 + React + TypeScript 项目脚手架
2. **db-layer** — 实现 SQLite 数据库层 (schema 设计、连接管理、migration)
3. **llm-gateway** — 实现 LLM 网关 (统一接口适配 OpenAI/Claude/Ollama，支持 streaming)
4. **chat-backend** — 实现对话后端逻辑 (对话管理、历史存储、Tauri commands)
5. **chat-ui** — 实现对话前端界面 (对话列表、消息展示、模型切换、Markdown 渲染)
6. **settings** — 实现设置模块 (API Key 管理、全局配置)
7. **doc-processor** — 实现文档处理器 (上传、解析 txt/md/pdf、文本分块)
8. **embedding-engine** — 实现 Embedding 引擎 (调用 Embedding API、向量存储与检索)
9. **rag-integration** — RAG 集成 (知识库管理 UI、对话中知识库检索注入)
10. **polish** — 整体打磨 (错误处理、加载状态、UI 美化、打包测试)

### 依赖关系

```
project-init ✅
  ├── db-layer
  │     ├── chat-backend
  │     └── doc-processor
  ├── llm-gateway
  │     └── chat-backend
  │           └── chat-ui
  ├── settings (依赖 db-layer)
  ├── embedding-engine (依赖 db-layer, llm-gateway)
  │     └── rag-integration (依赖 doc-processor, embedding-engine, chat-ui)
  └── polish (依赖所有)
```

## 关键 Rust Crates

- `tauri` — 桌面应用框架
- `rusqlite` — SQLite 绑定
- `reqwest` — HTTP 客户端 (调用 AI API)
- `serde` / `serde_json` — 序列化
- `tokio` — 异步运行时
- `pdf-extract` 或 `lopdf` — PDF 解析
- `text-splitter` — 文本分块
- `usearch` 或自定义 HNSW — 向量搜索

## 备注

- 优先保证核心体验流畅，功能可以逐步迭代
- API Key 需加密存储，不能明文保存
- 考虑离线场景：接入 Ollama 本地模型可完全离线使用
