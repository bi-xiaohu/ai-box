import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  DocumentInfo,
  listDocuments,
  uploadDocument,
  deleteDocument,
} from "../lib/api";

interface KnowledgeBaseProps {
  open: boolean;
  onClose: () => void;
}

export default function KnowledgeBase({
  open: isOpen,
  onClose,
}: KnowledgeBaseProps) {
  const [documents, setDocuments] = useState<DocumentInfo[]>([]);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (isOpen) {
      loadDocuments();
    }
  }, [isOpen]);

  async function loadDocuments() {
    try {
      const docs = await listDocuments();
      setDocuments(docs);
    } catch (e) {
      console.error("Failed to load documents:", e);
    }
  }

  async function handleUpload() {
    setError("");
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Documents",
            extensions: ["txt", "md", "pdf"],
          },
        ],
      });

      if (!selected) return;

      setUploading(true);
      await uploadDocument(selected);
      await loadDocuments();
    } catch (e) {
      setError(`Upload failed: ${e}`);
    } finally {
      setUploading(false);
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteDocument(id);
      setDocuments((prev) => prev.filter((d) => d.id !== id));
    } catch (e) {
      setError(`Delete failed: ${e}`);
    }
  }

  function formatSize(bytes: number | null): string {
    if (bytes === null) return "—";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-2xl mx-4 shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-800">
          <h2 className="text-lg font-semibold">📚 Knowledge Base</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white transition-colors cursor-pointer"
          >
            ✕
          </button>
        </div>

        {/* Body */}
        <div className="px-6 py-4">
          {/* Upload button */}
          <button
            onClick={handleUpload}
            disabled={uploading}
            className="w-full py-3 border-2 border-dashed border-gray-700 hover:border-blue-500 rounded-lg text-sm text-gray-400 hover:text-blue-400 transition-colors disabled:opacity-50 cursor-pointer mb-4"
          >
            {uploading
              ? "Uploading & generating embeddings..."
              : "📄 Click to upload document (txt, md, pdf)"}
          </button>

          {error && (
            <p className="text-red-400 text-sm mb-3">{error}</p>
          )}

          {/* Document list */}
          <div className="max-h-[50vh] overflow-y-auto space-y-2">
            {documents.length === 0 ? (
              <p className="text-gray-600 text-sm text-center py-8">
                No documents uploaded yet
              </p>
            ) : (
              documents.map((doc) => (
                <div
                  key={doc.id}
                  className="group flex items-center justify-between px-4 py-3 bg-gray-800 rounded-lg"
                >
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-white truncate">
                      {doc.filename}
                    </p>
                    <p className="text-xs text-gray-500">
                      {doc.file_type.toUpperCase()} · {formatSize(doc.file_size)}{" "}
                      · {new Date(doc.created_at).toLocaleDateString()}
                    </p>
                  </div>
                  <button
                    onClick={() => handleDelete(doc.id)}
                    className="opacity-0 group-hover:opacity-100 text-gray-500 hover:text-red-400 ml-3 transition-opacity cursor-pointer"
                  >
                    🗑
                  </button>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-3 border-t border-gray-800 text-xs text-gray-500">
          Documents are chunked and embedded for semantic search during conversations.
        </div>
      </div>
    </div>
  );
}
