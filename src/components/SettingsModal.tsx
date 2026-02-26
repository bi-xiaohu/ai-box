import { useEffect, useState, useRef, useCallback } from "react";
import {
  getSettings,
  setSetting,
  deleteSetting,
  copilotStartLogin,
  copilotPollLogin,
  copilotIsLoggedIn,
  copilotLogout,
} from "../lib/api";
import { openUrl } from "@tauri-apps/plugin-opener";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}

interface SettingField {
  key: string;
  label: string;
  placeholder: string;
  secret: boolean;
}

const FIELDS: SettingField[] = [
  {
    key: "openai_api_key",
    label: "OpenAI API Key",
    placeholder: "sk-...",
    secret: true,
  },
  {
    key: "openai_base_url",
    label: "OpenAI Base URL",
    placeholder: "https://api.openai.com/v1",
    secret: false,
  },
  {
    key: "claude_api_key",
    label: "Claude API Key",
    placeholder: "sk-ant-...",
    secret: true,
  },
  {
    key: "claude_base_url",
    label: "Claude Base URL",
    placeholder: "https://api.anthropic.com",
    secret: false,
  },
  {
    key: "ollama_host",
    label: "Ollama Host",
    placeholder: "http://localhost:11434",
    secret: false,
  },
];

export default function SettingsModal({
  open: isOpen,
  onClose,
  onSaved,
}: SettingsModalProps) {
  const [values, setValues] = useState<Record<string, string>>({});
  const [editValues, setEditValues] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  // Copilot login state
  const [copilotLoggedIn, setCopilotLoggedIn] = useState(false);
  const [copilotLoggingIn, setCopilotLoggingIn] = useState(false);
  const [, setDeviceCode] = useState("");
  const [userCode, setUserCode] = useState("");
  const pollRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const stopPolling = useCallback(() => {
    if (pollRef.current) {
      clearTimeout(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (isOpen) {
      getSettings()
        .then((s) => {
          setValues(s);
          setEditValues({});
          setMessage("");
        })
        .catch(console.error);
      copilotIsLoggedIn().then(setCopilotLoggedIn).catch(console.error);
    }
    return stopPolling;
  }, [isOpen, stopPolling]);

  async function handleCopilotLogin() {
    setCopilotLoggingIn(true);
    setMessage("");
    try {
      const resp = await copilotStartLogin();
      setDeviceCode(resp.device_code);
      setUserCode(resp.user_code);
      // Open browser for user to authorize
      await openUrl(resp.verification_uri);
      // Start polling with recursive setTimeout to avoid overlapping requests
      const interval = Math.max(resp.interval, 3) * 1000;
      const poll = async () => {
        try {
          const token = await copilotPollLogin(resp.device_code);
          if (token) {
            setCopilotLoggedIn(true);
            setCopilotLoggingIn(false);
            setDeviceCode("");
            setUserCode("");
            setMessage("GitHub Copilot connected!");
            onSaved();
            return;
          }
        } catch (err) {
          setCopilotLoggingIn(false);
          setMessage("Error: Login failed or expired. Try again.");
          return;
        }
        pollRef.current = setTimeout(poll, interval);
      };
      pollRef.current = setTimeout(poll, interval);
    } catch (e) {
      setCopilotLoggingIn(false);
      setMessage(`Error: ${e}`);
    }
  }

  async function handleCopilotLogout() {
    await copilotLogout();
    setCopilotLoggedIn(false);
    setMessage("GitHub Copilot disconnected.");
    onSaved();
  }

  async function handleSave() {
    setSaving(true);
    setMessage("");
    try {
      for (const [key, value] of Object.entries(editValues)) {
        if (value.trim()) {
          await setSetting(key, value.trim());
        } else {
          await deleteSetting(key);
        }
      }
      setMessage("Settings saved!");
      onSaved();
      // Reload settings to show masked values
      const s = await getSettings();
      setValues(s);
      setEditValues({});
    } catch (e) {
      setMessage(`Error: ${e}`);
    } finally {
      setSaving(false);
    }
  }

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-lg mx-4 shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-800">
          <h2 className="text-lg font-semibold">Settings</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white transition-colors cursor-pointer"
          >
            ✕
          </button>
        </div>

        {/* Body */}
        <div className="px-6 py-4 space-y-4 max-h-[60vh] overflow-y-auto">
          {FIELDS.map((field) => (
            <div key={field.key}>
              <label className="block text-sm text-gray-400 mb-1">
                {field.label}
              </label>
              <input
                type={field.secret ? "password" : "text"}
                placeholder={
                  values[field.key]
                    ? `Current: ${values[field.key]}`
                    : field.placeholder
                }
                value={editValues[field.key] ?? ""}
                onChange={(e) =>
                  setEditValues((prev) => ({
                    ...prev,
                    [field.key]: e.target.value,
                  }))
                }
                className="w-full bg-gray-800 text-white text-sm rounded-lg px-3 py-2.5 border border-gray-700 focus:outline-none focus:border-blue-500 placeholder-gray-500"
              />
            </div>
          ))}

          {/* GitHub Copilot Login */}
          <div className="pt-2 border-t border-gray-800">
            <label className="block text-sm text-gray-400 mb-2">
              GitHub Copilot
            </label>
            {copilotLoggedIn ? (
              <div className="flex items-center justify-between">
                <span className="text-sm text-green-400">✓ Connected</span>
                <button
                  onClick={handleCopilotLogout}
                  className="px-3 py-1.5 text-sm text-red-400 hover:text-red-300 border border-red-800 hover:border-red-600 rounded-lg transition-colors cursor-pointer"
                >
                  Disconnect
                </button>
              </div>
            ) : copilotLoggingIn ? (
              <div className="space-y-2">
                <p className="text-sm text-gray-300">
                  Enter code{" "}
                  <code className="bg-gray-800 px-2 py-0.5 rounded font-mono text-yellow-300 text-base">
                    {userCode}
                  </code>{" "}
                  in the browser window
                </p>
                <p className="text-xs text-gray-500">Waiting for authorization...</p>
              </div>
            ) : (
              <button
                onClick={handleCopilotLogin}
                className="px-4 py-2 bg-gray-800 hover:bg-gray-700 border border-gray-600 rounded-lg text-sm transition-colors cursor-pointer"
              >
                Login with GitHub
              </button>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-6 py-4 border-t border-gray-800">
          {message && (
            <span
              className={`text-sm ${message.startsWith("Error") ? "text-red-400" : "text-green-400"}`}
            >
              {message}
            </span>
          )}
          <div className="flex gap-2 ml-auto">
            <button
              onClick={onClose}
              className="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors cursor-pointer"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              disabled={saving || Object.keys(editValues).length === 0}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-40 rounded-lg text-sm font-medium transition-colors cursor-pointer"
            >
              {saving ? "Saving..." : "Save"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
