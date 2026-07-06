import { useState } from "react";
import { Clipboard, Check } from "lucide-react";
import { copyToClipboard } from "../utils/clipboard";

interface Props {
  text: string;
  variant?: "icon" | "full";
}

export default function CopyButton({ text, variant = "icon" }: Props) {
  const [copied, setCopied] = useState(false);

  async function handleCopy(e: React.MouseEvent) {
    e.stopPropagation();
    const ok = await copyToClipboard(text);
    if (ok) {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
  }

  if (variant === "full") {
    return (
      <button
        onClick={handleCopy}
        className="flex items-center gap-1 px-2 py-1 text-xs bg-surface-hover hover:bg-surface-border text-gray-300 rounded-md transition-colors whitespace-nowrap"
      >
        {copied ? <Check size={12} className="text-green-400" /> : <Clipboard size={12} />}
        {copied ? "Copied" : "Copy"}
      </button>
    );
  }

  return (
    <button
      onClick={handleCopy}
      className="text-gray-400 hover:text-gray-200 transition-colors flex-shrink-0"
      title="Copy command"
    >
      {copied ? <Check size={14} className="text-green-400" /> : <Clipboard size={14} />}
    </button>
  );
}
