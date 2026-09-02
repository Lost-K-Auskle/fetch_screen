import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

/**
 * 截图浮窗预览：截完图浮窗展示，供用户 置顶 / 穿透 / 关闭 / 删除。
 * 可拖动、缩放、滚轮放大；透明度可调；穿透模式点击落到下层窗口。
 */
interface PreviewPayload {
  path: string;
  data_url: string;
}

export default function PreviewWindow() {
  const [payload, setPayload] = useState<PreviewPayload | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [passthrough, setPassthrough] = useState(false);
  const [opacity, setOpacity] = useState(1.0);
  // 缩放/平移：图片以 fit 尺寸为基准，transform = translate(pan) scale(zoom)，原点在左上
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [natural, setNatural] = useState<{ w: number; h: number } | null>(null);
  const [fit, setFit] = useState<{ w: number; h: number } | null>(null);
  const [size, setSize] = useState({ w: window.innerWidth, h: window.innerHeight });
  // 拖拽方式: "left_drag" = 左键拖窗 / Shift+左键平移; "shift_drag" = 左键平移 / Shift+左键拖窗
  const [dragMode, setDragMode] = useState<'left_drag' | 'shift_drag'>('left_drag');
  // 无截图区域背景: "black" = 黑色 / "white" = 白色 / "hollow" = 镂空(透明)
  const [bgMode, setBgMode] = useState<'black' | 'white' | 'hollow'>('black');
  // 供 Delete 键等异步回调读取最新 payload（避免闭包抓到初始 null）
  const payloadRef = useRef<PreviewPayload | null>(null);
  useEffect(() => { payloadRef.current = payload; }, [payload]);

  useEffect(() => {
    invoke<PreviewPayload | null>('get_preview_payload')
      .then((p) => {
        if (!p) {
          getCurrentWindow().close().catch(() => {});
          return;
        }
        setPayload(p);
        const img = new Image();
        img.onload = () => {
          setLoaded(true);
          setNatural({ w: img.naturalWidth, h: img.naturalHeight });
          setSize({ w: window.innerWidth, h: window.innerHeight });
        };
        img.onerror = () => console.error('预览图加载失败');
        img.src = p.data_url;
      })
      .catch(console.error);

    // 读取拖拽方式 + 背景样式配置
    invoke<{ preview_drag_mode?: string; preview_bg_mode?: string }>('get_config')
      .then((c) => {
        if (c?.preview_drag_mode === 'shift_drag') setDragMode('shift_drag');
        if (c?.preview_bg_mode === 'white') setBgMode('white');
        else if (c?.preview_bg_mode === 'hollow') setBgMode('hollow');
        else setBgMode('black');
      })
      .catch(() => {});
  }, []);

  // 窗口尺寸变化时监听，重新适配
  useEffect(() => {
    const onResize = () => setSize({ w: window.innerWidth, h: window.innerHeight });
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  // 监听后端（穿透热键）触发的穿透状态变化
  useEffect(() => {
    let un: (() => void) | null = null;
    listen<boolean>('pin:passthrough', (e) => setPassthrough(e.payload)).then((fn) => { un = fn; });
    return () => { un?.(); };
  }, []);

  // 穿透/交互切换（穿透后点击会落到下层窗口，用全局热键 Ctrl+Shift+P 切回）
  const handlePassthrough = async () => {
    try {
      const next = await invoke<boolean>('set_pin_passthrough', { enabled: !passthrough });
      setPassthrough(next);
    } catch (err) {
      console.error('切换穿透失败:', err);
    }
  };

  // 根据容器尺寸 + 图片原始尺寸，计算 fit 基准尺寸并居中
  useEffect(() => {
    if (!natural) return;
    // cover：图片填满浮窗（超出部分裁剪，可缩放/方向键平移查看）
    const fs = Math.max(size.w / natural.w, size.h / natural.h);
    const fw = natural.w * fs;
    const fh = natural.h * fs;
    setFit({ w: fw, h: fh });
    setPan({ x: (size.w - fw) / 2, y: (size.h - fh) / 2 });
  }, [size, natural]);

  // 置顶：把当前浮窗切换为置顶/取消置顶（不新建窗口）
  const handlePin = async () => {
    try {
      await getCurrentWindow().setAlwaysOnTop(!pinned);
      setPinned(!pinned);
    } catch (err) {
      console.error('置顶失败:', err);
    }
  };

  // 关闭：保留文件
  const handleClose = () => {
    getCurrentWindow().close().catch(() => {});
  };

  // 删除：删除文件并关闭
  const handleDelete = async () => {
    const p = payloadRef.current;
    if (!p) return;
    try {
      await invoke('delete_image', { path: p.path });
    } catch (err) {
      console.error('删除失败:', err);
    }
    getCurrentWindow().close().catch(() => {});
  };

  // 选中浮窗后按 Delete 键快速删除（用 ref 读最新 payload，避免闭包捕获旧值）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Delete') return;
      e.preventDefault();
      handleDelete();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 单左键拖拽平移图片：记录按下起点与 pan 起点
  const dragRef = useRef<{ startX: number; startY: number; panX: number; panY: number } | null>(null);

  // 按下非按钮/滑块区域：
  // - left_drag（默认）: 单左键 → 拖窗; Shift+左键 → 平移
  // - shift_drag: 单左键 → 平移; Shift+左键 → 拖窗
  const handleDragMouseDown = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button, input')) return;
    if (e.button !== 0) return;
    const shiftDrags = dragMode === 'shift_drag';
    if (e.shiftKey === shiftDrags) {
      // 拖窗
      getCurrentWindow().startDragging().catch(() => {});
      return;
    }
    // 平移
    dragRef.current = { startX: e.clientX, startY: e.clientY, panX: pan.x, panY: pan.y };
    e.preventDefault();
  };

  // 单左键拖动中：实时更新图片平移位置
  const handleMouseMove = (e: React.MouseEvent) => {
    const d = dragRef.current;
    if (!d) return;
    setPan({ x: d.panX + (e.clientX - d.startX), y: d.panY + (e.clientY - d.startY) });
  };

  const handleMouseUp = () => {
    dragRef.current = null;
  };

  // 滚轮缩放图片（10%~10000%），以鼠标光标为原点（光标下的像素保持不动）
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    if (!fit) return;
    // 按比例缩放：每格 ±10%，这样高倍数（100x）也能较快滚到
    const factor = e.deltaY > 0 ? 0.9 : 1.1;
    const newZoom = Math.max(0.1, Math.min(100.0, +(zoom * factor).toFixed(2)));
    const cx = e.clientX;
    const cy = e.clientY;
    // 屏幕坐标 = pan + p*zoom；光标下图像点 p 在缩放后应仍在光标处
    const px = (cx - pan.x) / zoom;
    const py = (cy - pan.y) / zoom;
    setPan({ x: cx - px * newZoom, y: cy - py * newZoom });
    setZoom(newZoom);
  };

  // 方向键平移图片
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const step = 30;
      switch (e.key) {
        case 'ArrowUp': setPan((p) => ({ ...p, y: p.y + step })); break;
        case 'ArrowDown': setPan((p) => ({ ...p, y: p.y - step })); break;
        case 'ArrowLeft': setPan((p) => ({ ...p, x: p.x + step })); break;
        case 'ArrowRight': setPan((p) => ({ ...p, x: p.x - step })); break;
        default: return;
      }
      e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const bgColor = bgMode === 'black' ? '#000000' : bgMode === 'white' ? '#ffffff' : 'transparent';

  return (
    <div
      onMouseDown={handleDragMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onDoubleClick={handlePassthrough}
      onWheel={handleWheel}
      style={{
        width: '100vw', height: '100vh', position: 'relative',
        border: '1px solid rgba(255,255,255,0.15)', borderRadius: 6,
        overflow: 'hidden', userSelect: 'none', cursor: 'move',
        background: bgColor,
      }}
    >
      {/* 镂空层：无背景，只有图片，透明度由滑块控制 → 淡出后直接露出下层 */}
      <div style={{
        position: 'absolute', inset: 0, overflow: 'hidden',
        background: 'transparent', opacity, transition: 'opacity 0.1s',
      }}>
        {!loaded ? (
          <span style={{ color: '#666', fontSize: 12, position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%,-50%)' }}>加载中...</span>
        ) : (
          fit && (
            <img
              src={payload!.data_url}
              alt="preview"
              draggable={false}
              style={{
                position: 'absolute', left: 0, top: 0,
                width: fit.w, height: fit.h,
                transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
                transformOrigin: '0 0',
                pointerEvents: 'none',
              }}
            />
          )
        )}
      </div>

      {/* 缩放指示 */}
      {loaded && (
        <div style={{
          position: 'absolute', bottom: 22, left: 6, zIndex: 5,
          color: 'rgba(255,255,255,0.5)', fontSize: 10, pointerEvents: 'none',
        }}>
          {Math.round(zoom * 100)}%
        </div>
      )}

      {/* 透明度拖拉条 */}
      {loaded && (
        <div style={{
          position: 'absolute', bottom: 2, left: '50%', transform: 'translateX(-50%)',
          display: 'flex', alignItems: 'center', gap: 4, zIndex: 5,
          background: 'rgba(0,0,0,0.35)', padding: '1px 6px', borderRadius: 8,
        }}>
          <input
            type="range"
            min={0.2}
            max={1}
            step={0.05}
            value={opacity}
            onChange={(e) => setOpacity(Number(e.target.value))}
            style={{ width: 90, accentColor: '#5b8cff', cursor: 'pointer', margin: 0 }}
          />
          <span style={{ color: 'rgba(255,255,255,0.6)', fontSize: 9 }}>{Math.round(opacity * 100)}%</span>
        </div>
      )}

      {/* 穿透提示 */}
      {passthrough && (
        <div style={{
          position: 'absolute', top: 6, left: 6, zIndex: 5,
          color: '#9fc3ff', fontSize: 10, pointerEvents: 'none',
          background: 'rgba(0,0,0,0.5)', padding: '2px 6px', borderRadius: 4,
        }}>
          穿透中 · Ctrl+Shift+P 恢复
        </div>
      )}

      {/* 右上角小图标按钮 */}
      <div style={{ position: 'absolute', top: 6, right: 6, display: 'flex', gap: 4, zIndex: 5 }}>
        <MiniBtn title={passthrough ? '穿透中（Ctrl+Shift+P 恢复）' : '鼠标穿透'} onClick={handlePassthrough} active={passthrough}>🖱️</MiniBtn>
        <MiniBtn title={pinned ? '取消置顶' : '置顶'} onClick={handlePin} active={pinned}>📌</MiniBtn>
        <MiniBtn title="关闭" onClick={handleClose}>✕</MiniBtn>
        <MiniBtn title="删除" onClick={handleDelete}>🗑</MiniBtn>
      </div>
    </div>
  );
}

function MiniBtn({ title, onClick, active, children }: {
  title: string;
  onClick: () => void;
  active?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      title={title}
      onClick={(e) => { e.stopPropagation(); onClick(); }}
      style={{
        width: 22, height: 22, display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: active ? 'rgba(80,140,255,0.35)' : 'rgba(255,255,255,0.08)',
        border: active ? '1px solid rgba(120,170,255,0.6)' : '1px solid rgba(255,255,255,0.15)',
        borderRadius: 4, color: active ? '#fff' : '#ddd', fontSize: 11, cursor: 'pointer', padding: 0,
        lineHeight: 1,
      }}
    >
      {children}
    </button>
  );
}
