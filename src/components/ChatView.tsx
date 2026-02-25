import { useEffect, useRef, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  ChatStreamEvent,
  getMessages,
  Message,
  ModelInfo,
  sendMessage,
} from "../lib/api";

interface ChatViewProps {
  conversationId: string | null;
  models: ModelInfo[];
  currentModel: string;
  onModelChange: (model: string) => void;
}

export default function ChatView({
  conversationId,
  models,
  currentModel,
  onModelChange,
}: ChatViewProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [streamContent, setStreamContent] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Load messages when conversation changes
  useEffect(() => {
    if (!conversationId) {
      setMessages([]);
      return;
    }
    getMessages(conversationId).then(setMessages).catch(console.error);
  }, [conversationId]);

  // Listen to streaming events
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    listen<ChatStreamEvent>("chat-stream", (event) => {
      const { conversation_id, delta, done } = event.payload;
      if (conversation_id !== conversationId) return;

      if (done) {
        // Stream complete — reload messages from DB
        if (conversationId) {
          getMessages(conversationId).then((msgs) => {
            setMessages(msgs);
            setStreamContent("");
            setStreaming(false);
          });
        }
      } else {
        setStreamContent((prev) => prev + delta);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [conversationId]);

  // Auto-scroll to bottom
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamContent]);

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height =
        Math.min(textareaRef.current.scrollHeight, 200) + "px";
    }
  }, [input]);

  async function handleSend() {
    if (!input.trim() || !conversationId || streaming) return;

    const content = input.trim();
    setInput("");
    setStreaming(true);
    setStreamContent("");

    // Optimistically add user message
    const userMsg: Message = {
      id: "temp-" + Date.now(),
      conversation_id: conversationId,
      role: "user",
      content,
      created_at: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);

    try {
      await sendMessage(conversationId, content, currentModel);
    } catch (e) {
      console.error("Send failed:", e);
      setStreaming(false);
      setStreamContent("");
      // Show error as assistant message
      setMessages((prev) => [
        ...prev,
        {
          id: "error-" + Date.now(),
          conversation_id: conversationId,
          role: "assistant",
          content: `⚠️ Error: ${e}`,
          created_at: new Date().toISOString(),
        },
      ]);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  if (!conversationId) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-500">
        <div className="text-center">
          <p className="text-5xl mb-4">🤖</p>
          <p className="text-lg">Select or create a conversation to start</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col h-full">
      {/* Top bar with model selector */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-800 bg-gray-900/50">
        <span className="text-sm text-gray-400">AI-Box Chat</span>
        <select
          value={currentModel}
          onChange={(e) => onModelChange(e.target.value)}
          className="bg-gray-800 text-gray-300 text-sm rounded-md px-3 py-1.5 border border-gray-700 focus:outline-none focus:border-blue-500"
        >
          {models.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name} ({m.provider})
            </option>
          ))}
        </select>
      </div>

      {/* Messages area */}
      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-4">
        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}

        {/* Streaming indicator */}
        {streaming && streamContent && (
          <div className="flex justify-start">
            <div className="max-w-[80%] px-4 py-3 rounded-2xl bg-gray-800 text-gray-100">
              <div className="prose prose-invert prose-sm max-w-none">
                <ReactMarkdown remarkPlugins={[remarkGfm]}>
                  {streamContent}
                </ReactMarkdown>
              </div>
            </div>
          </div>
        )}

        {streaming && !streamContent && (
          <div className="flex justify-start">
            <div className="px-4 py-3 rounded-2xl bg-gray-800 text-gray-400">
              <span className="animate-pulse">Thinking...</span>
            </div>
          </div>
        )}

        <div ref={bottomRef} />
      </div>

      {/* Input area */}
      <div className="px-4 py-3 border-t border-gray-800 bg-gray-900/50">
        <div className="flex items-end gap-2 max-w-4xl mx-auto">
          <textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a message... (Enter to send, Shift+Enter for newline)"
            rows={1}
            className="flex-1 bg-gray-800 text-white rounded-xl px-4 py-3 text-sm resize-none border border-gray-700 focus:outline-none focus:border-blue-500 placeholder-gray-500"
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || streaming}
            className="px-4 py-3 bg-blue-600 hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed rounded-xl text-sm font-medium transition-colors cursor-pointer"
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}

function MessageBubble({ message }: { message: Message }) {
  const isUser = message.role === "user";
  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[80%] px-4 py-3 rounded-2xl ${
          isUser
            ? "bg-blue-600 text-white"
            : "bg-gray-800 text-gray-100"
        }`}
      >
        {isUser ? (
          <p className="text-sm whitespace-pre-wrap">{message.content}</p>
        ) : (
          <div className="prose prose-invert prose-sm max-w-none">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {message.content}
            </ReactMarkdown>
          </div>
        )}
      </div>
    </div>
  );
}
