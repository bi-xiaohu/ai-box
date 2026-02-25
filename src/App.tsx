import { useEffect, useState } from "react";
import "./App.css";
import Sidebar from "./components/Sidebar";
import ChatView from "./components/ChatView";
import SettingsModal from "./components/SettingsModal";
import KnowledgeBase from "./components/KnowledgeBase";
import {
  Conversation,
  ModelInfo,
  getAvailableModels,
  listConversations,
} from "./lib/api";

function App() {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentId, setCurrentId] = useState<string | null>(null);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [currentModel, setCurrentModel] = useState("ollama/llama3");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [kbOpen, setKbOpen] = useState(false);

  // Load conversations and models on mount
  useEffect(() => {
    listConversations().then(setConversations).catch(console.error);
    loadModels();
  }, []);

  function loadModels() {
    getAvailableModels()
      .then((m) => {
        setModels(m);
        if (m.length > 0 && !m.find((x) => x.id === currentModel)) {
          setCurrentModel(m[0].id);
        }
      })
      .catch(console.error);
  }

  function handleNewConversation(conv: Conversation) {
    setConversations((prev) => [conv, ...prev]);
    setCurrentId(conv.id);
  }

  function handleDeleteConversation(id: string) {
    setConversations((prev) => prev.filter((c) => c.id !== id));
    if (currentId === id) {
      setCurrentId(null);
    }
  }

  return (
    <main className="flex h-screen bg-gray-950 text-white overflow-hidden">
      <Sidebar
        conversations={conversations}
        currentId={currentId}
        onSelect={setCurrentId}
        onNew={handleNewConversation}
        onDelete={handleDeleteConversation}
        onOpenSettings={() => setSettingsOpen(true)}
        onOpenKnowledgeBase={() => setKbOpen(true)}
        model={currentModel}
      />
      <ChatView
        conversationId={currentId}
        models={models}
        currentModel={currentModel}
        onModelChange={setCurrentModel}
      />
      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onSaved={loadModels}
      />
      <KnowledgeBase open={kbOpen} onClose={() => setKbOpen(false)} />
    </main>
  );
}

export default App;
