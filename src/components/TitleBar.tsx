import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

const isMac = navigator.platform.toLowerCase().includes("mac");

function useMaximized() {
  const [isMaximized, setIsMaximized] = useState(false);
  useEffect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    win.onResized(() => {
      win.isMaximized().then(setIsMaximized);
    }).then(fn => {
      unlisten = fn;
    });
    win.isMaximized().then(setIsMaximized);
    return () => unlisten?.();
  }, []);
  return isMaximized;
}

function TrafficLights() {
  const isMaximized = useMaximized();

  return (
    <div className="flex items-center gap-2 group">
      <button
        className="w-3 h-3 rounded-full bg-[#ff5f57] hover:bg-[#ff5f57] flex items-center justify-center"
        onClick={() => getCurrentWindow().close()}
        title="关闭"
      >
        <svg className="opacity-0 group-hover:opacity-100" width="6" height="6" viewBox="0 0 6 6" fill="none">
          <path d="M1.2 1.2 L4.8 4.8 M4.8 1.2 L1.2 4.8" stroke="#4d0000" strokeWidth="1.3" strokeLinecap="round"/>
        </svg>
      </button>
      <button
        className="w-3 h-3 rounded-full bg-[#febc2e] hover:bg-[#febc2e] flex items-center justify-center"
        onClick={() => getCurrentWindow().minimize()}
        title="最小化"
      >
        <svg className="opacity-0 group-hover:opacity-100" width="6" height="6" viewBox="0 0 6 6" fill="none">
          <line x1="1.2" y1="3" x2="4.8" y2="3" stroke="#4d3300" strokeWidth="1.3" strokeLinecap="round"/>
        </svg>
      </button>
      <button
        className="w-3 h-3 rounded-full bg-[#28c840] hover:bg-[#28c840] flex items-center justify-center"
        onClick={() => getCurrentWindow().toggleMaximize()}
        title={isMaximized ? "还原" : "最大化"}
      >
        {isMaximized ? (
          <svg className="opacity-0 group-hover:opacity-100" width="6" height="6" viewBox="0 0 6 6" fill="none">
            <path d="M1 1 L3.5 1 L1 3.5 Z" fill="#003d00"/>
            <path d="M5 5 L2.5 5 L5 2.5 Z" fill="#003d00"/>
          </svg>
        ) : (
          <svg className="opacity-0 group-hover:opacity-100" width="6" height="6" viewBox="0 0 6 6" fill="none">
            <path d="M1 5 L1 2.5 L3.5 5 Z" fill="#003d00"/>
            <path d="M5 1 L5 3.5 L2.5 1 Z" fill="#003d00"/>
          </svg>
        )}
      </button>
    </div>
  );
}

function WindowControls() {
  const isMaximized = useMaximized();

  const btnBase = "w-[46px] h-full flex items-center justify-center text-gray-400 hover:text-gray-100 hover:bg-white/10 transition-colors";

  return (
    <div className="flex h-full">
      <button className={btnBase} onClick={() => getCurrentWindow().minimize()} title="最小化">
        <svg width="12" height="12" viewBox="0 0 12 12"><rect y="5" width="12" height="1.5" fill="currentColor"/></svg>
      </button>
      <button className={btnBase} onClick={() => getCurrentWindow().toggleMaximize()} title={isMaximized ? "还原" : "最大化"}>
        {isMaximized ? (
          <svg width="12" height="12" viewBox="0 0 12 12">
            <rect x="2.5" y="0.5" width="9" height="9" rx="1" fill="none" stroke="currentColor" strokeWidth="1.2"/>
            <rect x="0.5" y="2.5" width="9" height="9" rx="1" fill="currentColor" stroke="currentColor" strokeWidth="1.2"/>
          </svg>
        ) : (
          <svg width="12" height="12" viewBox="0 0 12 12">
            <rect x="0.5" y="0.5" width="11" height="11" rx="1" fill="none" stroke="currentColor" strokeWidth="1.2"/>
          </svg>
        )}
      </button>
      <button className={`${btnBase} hover:bg-red-500 hover:text-white`} onClick={() => getCurrentWindow().close()} title="关闭">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" strokeWidth="1.2"/>
          <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" strokeWidth="1.2"/>
        </svg>
      </button>
    </div>
  );
}

export default function TitleBar() {
  function handleDoubleClick() {
    getCurrentWindow().toggleMaximize();
  }

  return (
    <div
      className="h-9 bg-surface flex items-center shrink-0 relative border-b border-surface-border"
      data-tauri-drag-region
    >
      {isMac ? (
        <>
          <div className="w-[76px] shrink-0 flex items-center pl-3">
            <TrafficLights />
          </div>
          <div className="flex-1 h-full" data-tauri-drag-region onDoubleClick={handleDoubleClick} />
          <div className="w-[76px] shrink-0" data-tauri-drag-region onDoubleClick={handleDoubleClick} />
          <span className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 text-xs text-gray-500 font-medium tracking-wide pointer-events-none select-none">
            OpenSession
          </span>
        </>
      ) : (
        <>
          <span className="text-xs text-gray-500 font-medium tracking-wide px-3 select-none" data-tauri-drag-region>
            OpenSession
          </span>
          <div className="flex-1 h-full" data-tauri-drag-region onDoubleClick={handleDoubleClick} />
          <WindowControls />
        </>
      )}
    </div>
  );
}