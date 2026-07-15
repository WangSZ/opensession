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
          <div className="w-[76px] shrink-0" data-tauri-drag-region onDoubleClick={handleDoubleClick} />
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