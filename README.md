# AI-Box

个人 AI 集成桌面应用 — 基于 Tauri v2 + Rust + React

## 功能

- 🤖 **多模型 AI 对话** — 接入 OpenAI / Claude / Ollama（本地模型），支持流式输出
- 📚 **RAG 知识库** — 上传文档（txt/md/pdf），自动分块 & Embedding，语义检索
- ⚙️ **灵活配置** — API Key 管理、自定义 Base URL、模型切换
- 🖥️ **本地运行** — 数据全部存在本地 SQLite，隐私有保障

## 技术栈

| 层面 | 选型 |
|------|------|
| 桌面框架 | Tauri v2 |
| 后端 | Rust |
| 前端 | React + TypeScript |
| 样式 | Tailwind CSS v4 |
| 存储 | SQLite |
| 向量搜索 | 内嵌余弦相似度 |

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建发布版
npm run tauri build
```

## 项目结构

```
src/                    # React 前端
├── components/         # UI 组件（Sidebar, ChatView, Settings, KnowledgeBase）
├── lib/api.ts          # Tauri invoke 封装
└── App.tsx             # 主入口

src-tauri/src/          # Rust 后端
├── commands/           # Tauri commands（chat, settings, knowledge）
├── db/                 # SQLite 数据库层
├── llm/                # LLM 网关（OpenAI, Claude, Ollama）
├── doc_processor.rs    # 文档解析 & 分块
├── embedding.rs        # Embedding 生成 & 向量搜索
└── lib.rs              # 应用入口
```

## 使用

1. 启动后点击 **⚙ Settings** 配置 API Key
2. 点击 **+ New Chat** 创建对话，选择模型聊天
3. 点击 **📚 Knowledge Base** 上传文档构建知识库

## License

MIT
