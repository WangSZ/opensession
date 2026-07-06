import { useEffect, useRef, useState, useLayoutEffect } from "react";
import type { ReactNode } from "react";

export interface MenuItem {
  type: "item" | "separator";
  label?: string;
  icon?: ReactNode;
  onClick?: () => void;
  danger?: boolean;
  submenu?: MenuItem[];
  onContextMenu?: () => void;
  disabled?: boolean;
  title?: string;
  checked?: boolean;
}

interface Props {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

const PADDING = 8;

export default function ContextMenu({ x, y, items, onClose }: Props) {
  const [openSubmenu, setOpenSubmenu] = useState<number | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  useLayoutEffect(() => {
    if (menuRef.current) {
      const rect = menuRef.current.getBoundingClientRect();
      setPos({
        x: Math.max(PADDING, Math.min(x, window.innerWidth - rect.width - PADDING)),
        y: Math.max(PADDING, Math.min(y, window.innerHeight - rect.height - PADDING)),
      });
    }
  }, [x, y]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-50 bg-black/20"
      onClick={onClose}
    >
      <div
        className="fixed"
        style={{ left: pos.x, top: pos.y, zIndex: 51 }}
        onClick={e => e.stopPropagation()}
      >
        <div
          ref={menuRef}
          className="bg-surface-card border border-white/10 rounded-lg shadow-2xl shadow-black/40 py-1 min-w-[180px] animate-context-menu"
        >
        {items.map((item, i) => {
          if (item.type === "separator") {
            return <div key={i} className="h-px bg-surface-border my-1" />;
          }
          return (
            <ContextMenuItem
              key={i}
              item={item}
              index={i}
              isSubmenuOpen={openSubmenu === i}
              onToggleSubmenu={() => setOpenSubmenu(openSubmenu === i ? null : i)}
              onClose={onClose}
            />
          );
        })}
        </div>
      </div>
    </div>
  );
}

function ContextMenuItem({
  item, index, isSubmenuOpen, onToggleSubmenu, onClose,
}: {
  item: MenuItem;
  index: number;
  isSubmenuOpen: boolean;
  onToggleSubmenu: () => void;
  onClose: () => void;
}) {
  const itemRef = useRef<HTMLDivElement>(null);
  const [submenuPos, setSubmenuPos] = useState<{ x: number; y: number } | null>(null);

  function handleClick() {
    if (item.submenu) {
      onToggleSubmenu();
    } else {
      item.onClick?.();
      onClose();
    }
  }

  function handleContextMenu(e: React.MouseEvent) {
    if (item.onContextMenu && item.type === "item") {
      e.preventDefault();
      item.onContextMenu();
      onClose();
    }
  }

  useEffect(() => {
    if (isSubmenuOpen && itemRef.current) {
      const rect = itemRef.current.getBoundingClientRect();
      let sx = rect.right;
      let sy = rect.top;
      if (sx + 200 > window.innerWidth) sx = rect.left - 200;
      if (sy + 300 > window.innerHeight) sy = window.innerHeight - 300;
      setSubmenuPos({ x: sx, y: sy });
    } else {
      setSubmenuPos(null);
    }
  }, [isSubmenuOpen]);

  return (
    <div className="relative">
      <div
        ref={itemRef}
        title={item.title}
        className={`flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer transition-colors ${
          item.danger
            ? "text-red-400 hover:bg-red-500/10"
            : "text-gray-200 hover:bg-surface-hover"
        } ${item.disabled ? "opacity-40 pointer-events-none" : ""}`}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
      >
        {item.checked !== undefined && (
          <span className="w-4 text-indigo-400">{item.checked ? "✓" : ""}</span>
        )}
        {item.checked === undefined && item.icon && (
          <span className="w-4 flex-shrink-0">{item.icon}</span>
        )}
        <span className="flex-1">{item.label}</span>
        {item.submenu && (
          <span className="text-gray-500 text-xs">▸</span>
        )}
      </div>
      {isSubmenuOpen && submenuPos && item.submenu && (
        <div
          className="fixed z-50 bg-surface-card border border-white/10 rounded-lg shadow-2xl shadow-black/40 py-1 min-w-[160px] animate-context-menu"
          style={{ left: submenuPos.x, top: submenuPos.y }}
        >
          {item.submenu.map((sub, j) => {
            if (sub.type === "separator") {
              return <div key={j} className="h-px bg-surface-border my-1" />;
            }
            return (
              <div
                key={j}
                title={sub.title}
                className={`flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer transition-colors ${
                  sub.danger
                    ? "text-red-400 hover:bg-red-500/10"
                    : "text-gray-200 hover:bg-surface-hover"
                } ${sub.disabled ? "opacity-40 pointer-events-none" : ""}`}
                onClick={() => { sub.onClick?.(); onClose(); }}
                onContextMenu={e => {
                  if (sub.onContextMenu) {
                    e.preventDefault();
                    sub.onContextMenu();
                    onClose();
                  }
                }}
              >
                {sub.checked !== undefined && (
                  <span className="w-4 text-indigo-400">{sub.checked ? "✓" : ""}</span>
                )}
                {sub.checked === undefined && sub.icon && (
                  <span className="w-4 flex-shrink-0">{sub.icon}</span>
                )}
                <span className="flex-1">{sub.label}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
