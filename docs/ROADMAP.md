# AI-Box Roadmap

## Phase 1 — MVP ✅ 已完成

- 多模型 AI 对话（OpenAI / Claude / Ollama，流式输出）
- SQLite 数据持久化（对话、消息、文档、设置）
- RAG 知识库（文档上传、分块、Embedding、向量搜索）
- 设置管理（API Key、Base URL、模型配置）
- 桌面应用打包（Tauri v2 + React + Tailwind CSS）

## Phase 2 — 核心体验增强

### P0 — 必须尽快完成

- **RAG 对话注入** — 聊天时自动检索知识库相关 chunks，注入 system prompt，实现基于文档的问答
- **API Key 加密存储** — 从 SQLite 明文迁移到 `tauri-plugin-stronghold` 或系统 Keyring

### P1 — 重要改进

- **Ollama 模型自动发现** — 调用 `GET /api/tags` 动态获取已安装的本地模型列表
- **对话标题自动生成** — 首条消息后调用 LLM 生成摘要标题
- **代码高亮** — 集成 highlight.js 或 shiki，Markdown 代码块语法着色
- **错误提示优化** — 前端 toast 通知替代 console.error，展示友好的错误信息
- **对话搜索** — 支持按关键词搜索历史对话

## Phase 3 — 多模态 AI 能力

### P2 — 新 AI 功能

- **图片生成** — 接入 DALL-E / Stable Diffusion API，独立画图面板
- **语音交互** — Whisper 语音转文字 + edge-tts 文字转语音
- **OCR 文档识别** — 扩展 doc_processor 支持图片和扫描 PDF
- **导出功能** — 对话导出为 Markdown / PDF

## Phase 4 — 架构演进

### P3 — 长期目标

- **插件系统** — 定义插件 API，支持第三方扩展新的 AI 功能
- **多窗口 / 标签页** — 同时打开多个对话窗口
- **向量数据库升级** — 文档量增大后从暴力余弦搜索迁移到 HNSW 索引
- **多语言 Embedding** — 支持中文优化的 Embedding 模型
- **同步与备份** — 对话和知识库云端备份 / 多设备同步
