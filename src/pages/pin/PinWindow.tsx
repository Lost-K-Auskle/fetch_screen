import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

/**
 * 贴图窗口 — 默认鼠标穿透模式
 * 双击切换为交互模式（可拖拽、缩放、旋转、调整透明度）
 */
export default function PinWindow() {
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [interactive, setInteractive] = useState(false);
  const [scale, setScale] = useState(1.0);
  const [opacity, setOpacity] = useState(1.0);
  const [rotation, setRotation] = useState(0);
  const imgRef = useRef<HTMLImageElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // 监听图片加载事件
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    listen<string>('pin:load', (event) => {
      const path = event.payload;
      setImageSrc(`asset://localhost/${encodeURIComponent(path)}`);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  // 监听交互模式切换
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    listen<boolean>('pin:interaction_mode', (event) => {
      setInteractive(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => { unlisten?.(); };
  }, []);

  // 双击切换交互/穿透模式
  const handleDoubleClick = useCallback(async () => {
    try {
      // 通过 Tauri 窗口 label 推断 pin_id (格式: pin_UUID)
      // 简化处理: 触发 toggle 事件
      const newMode = !interactive;
      setInteractive(newMode);
    } catch (err) {
      console.error('切换交互模式失败:', err);
    }
  }, [interactive]);

  // 滚轮缩放 (交互模式)
  const handleWheel = useCallback((e: React.WheelEvent) => {
    if (!interactive) return;
    e.preventDefault();

    if (e.ctrlKey) {
      // Ctrl+滚轮: 透明度
      const delta = e.deltaY > 0 ? -0.05 : 0.05;
      const newOpacity = Math.max(0.1, Math.min(1.0, opacity + delta));
      setOpacity(newOpacity);
      invoke('update_pin_opacity', { pinId: '', alpha: newOpacity }).catch(console.error);
    } else {
      // 滚轮: 缩放 10%-800%
      const delta = e.deltaY > 0 ? -0.05 : 0.05;
      const newScale = Math.max(0.1, Math.min(8.0, scale + delta));
      setScale(newScale);
    }
  }, [interactive, opacity, scale]);

  // 键盘快捷键 (交互模式)
  useEffect(() => {
    if (!interactive) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key) {
        case '1':
          setRotation((r) => r - 90);
          break;
        case '2':
          setRotation((r) => r + 90);
          break;
        case '3':
          // 水平翻转
          if (imgRef.current) {
            const current = imgRef.current.style.transform.includes('scaleX(-1)');
            imgRef.current.style.transform = current ? '' : 'scaleX(-1)';
          }
          break;
        case '4':
          // 垂直翻转
          if (imgRef.current) {
            const current = imgRef.current.style.transform.includes('scaleY(-1)');
            imgRef.current.style.transform = current ? '' : 'scaleY(-1)';
          }
          break;
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [interactive]);

  if (!imageSrc) {
    return (
      <div style={{
        width: '100vw', height: '100vh',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: 'transparent', color: '#888',
      }}>
        加载中...
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      style={{
        width: '100vw',
        height: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'transparent',
        opacity,
        cursor: interactive ? 'move' : 'default',
        userSelect: 'none',
        WebkitUserSelect: 'none',
      }}
      onDoubleClick={handleDoubleClick}
      onWheel={handleWheel}
    >
      <img
        ref={imgRef}
        src={imageSrc}
        alt="pin"
        draggable={false}
        style={{
          maxWidth: '100%',
          maxHeight: '100%',
          transform: `scale(${scale}) rotate(${rotation}deg)`,
          transition: interactive ? 'transform 0.1s ease, opacity 0.05s' : 'none',
          pointerEvents: 'none',
        }}
      />
      {/* 交互模式指示器 */}
      {interactive && (
        <div style={{
          position: 'fixed',
          bottom: 4,
          right: 4,
          background: 'rgba(0,0,0,0.6)',
          color: '#fff',
          padding: '2px 8px',
          borderRadius: 4,
          fontSize: 10,
          pointerEvents: 'none',
        }}>
          {Math.round(scale * 100)}% · α{Math.round(opacity * 100)}% · 双击退出
        </div>
      )}
    </div>
  );
}
