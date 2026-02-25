import { invoke } from "@tauri-apps/api/core";

// ── Types ──

export interface Conversation {
  id: string;
  title: string;
  model: string | null;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: string;
}

export interface ModelInfo {
  id: string;
  name: string;
  provider: string;
}

export interface ChatStreamEvent {
  conversation_id: string;
  delta: string;
  done: boolean;
}

// ── Chat API ──

export async function createConversation(
  title: string,
  model?: string
): Promise<Conversation> {
  return invoke("create_conversation", { title, model });
}

export async function listConversations(): Promise<Conversation[]> {
  return invoke("list_conversations");
}

export async function deleteConversation(id: string): Promise<void> {
  return invoke("delete_conversation", { id });
}

export async function renameConversation(
  id: string,
  title: string
): Promise<void> {
  return invoke("rename_conversation", { id, title });
}

export async function getMessages(conversationId: string): Promise<Message[]> {
  return invoke("get_messages", { conversationId });
}

export async function sendMessage(
  conversationId: string,
  content: string,
  model: string
): Promise<Message> {
  return invoke("send_message", { conversationId, content, model });
}

// ── Settings API ──

export async function getSettings(): Promise<Record<string, string>> {
  return invoke("get_settings");
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke("set_setting", { key, value });
}

export async function deleteSetting(key: string): Promise<void> {
  return invoke("delete_setting", { key });
}

export async function getAvailableModels(): Promise<ModelInfo[]> {
  return invoke("get_available_models");
}

// ── Knowledge Base API ──

export interface DocumentInfo {
  id: string;
  filename: string;
  file_type: string;
  file_path: string;
  file_size: number | null;
  created_at: string;
}

export interface ChunkInfo {
  id: string;
  content: string;
  chunk_index: number;
  score: number | null;
}

export async function listDocuments(): Promise<DocumentInfo[]> {
  return invoke("list_documents");
}

export async function uploadDocument(filePath: string): Promise<DocumentInfo> {
  return invoke("upload_document", { filePath });
}

export async function deleteDocument(id: string): Promise<void> {
  return invoke("delete_document", { id });
}

export async function searchKnowledgeBase(
  query: string,
  topK?: number
): Promise<ChunkInfo[]> {
  return invoke("search_knowledge_base", { query, topK });
}
