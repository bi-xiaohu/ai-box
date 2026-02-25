import { useState } from "react";
import {
  Conversation,
  createConversation,
  deleteConversation,
} from "../lib/api";

interface SidebarProps {
  conversations: Conversation[];
  currentId: string | null;
  onSelect: (id: string) => void;
  onNew: (conv: Conversation) => void;
  onDelete: (id: string) => void;
  onOpenSettings: () => void;
  onOpenKnowledgeBase: () => void;
  model: string;
}

export default function Sidebar({
  conversations,
  currentId,
  onSelect,
  onNew,
  onDelete,
  onOpenSettings,
  onOpenKnowledgeBase,
  model,
}: SidebarProps) {
  const [loading, setLoading] = useState(false);

  async function handleNew() {
    setLoading(true);
    try {
      const conv = await createConversation("New Chat", model);
      onNew(conv);
    } catch (e) {
      console.error("Failed to create conversation:", e);
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(e: React.MouseEvent, id: string) {
    e.stopPropagation();
    try {
      await deleteConversation(id);
      onDelete(id);
    } catch (e) {
      console.error("Failed to delete:", e);
    }
  }

  return (
    <aside className="w-64 bg-gray-900 border-r border-gray-800 flex flex-col h-full">
      {/* Header */}
      <div className="p-3 border-b border-gray-800">
        <button
          onClick={handleNew}
          disabled={loading}
          className="w-full py-2 px-3 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors cursor-pointer"
        >
          + New Chat
        </button>
      </div>

      {/* Conversation list */}
      <div className="flex-1 overflow-y-auto">
        {conversations.map((conv) => (
          <div
            key={conv.id}
            onClick={() => onSelect(conv.id)}
            className={`group flex items-center justify-between px-3 py-2.5 cursor-pointer text-sm border-b border-gray-800/50 transition-colors ${
              conv.id === currentId
                ? "bg-gray-800 text-white"
                : "text-gray-400 hover:bg-gray-800/50 hover:text-gray-200"
            }`}
          >
            <span className="truncate flex-1">{conv.title}</span>
            <button
              onClick={(e) => handleDelete(e, conv.id)}
              className="opacity-0 group-hover:opacity-100 text-gray-500 hover:text-red-400 ml-2 transition-opacity cursor-pointer"
            >
              ✕
            </button>
          </div>
        ))}
        {conversations.length === 0 && (
          <p className="text-gray-600 text-xs text-center mt-8">
            No conversations yet
          </p>
        )}
      </div>

      {/* Footer */}
      <div className="p-3 border-t border-gray-800 space-y-1">
        <button
          onClick={onOpenKnowledgeBase}
          className="w-full py-2 px-3 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg text-sm transition-colors cursor-pointer"
        >
          📚 Knowledge Base
        </button>
        <button
          onClick={onOpenSettings}
          className="w-full py-2 px-3 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg text-sm transition-colors cursor-pointer"
        >
          ⚙ Settings
        </button>
      </div>
    </aside>
  );
}
