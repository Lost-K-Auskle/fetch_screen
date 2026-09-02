import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

interface ScrollFramePayload {
  x: number; y: number; width: number; height: number;
  desktop_width: number; desktop_height: number;
}

interface ScrollProgress {
  frame_count: number;
  total_height: number;
  max_length: number;
  preview_data_url?: string | null;
}

/**
 * 滚动截图的选区边框浮层 —— 全屏透明、点击穿透。
 * - 选区内部画半透明遮罩（不完全镂空），明确告知正在截取的区域
 * - 选区右侧实时预览当前拼接结果（随 scroll:progress 更新）
 * 滚动捕获期间持续显示；捕获完成/出错时自动关闭。
 */
export default function ScrollRegionFrame() {
  const [payload, setPayload] = useState<ScrollFramePayload | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const previewRef = useRef<HTMLImageElement | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);

  useEffect(() => {
    invoke<ScrollFramePayload | null>('get_scroll_frame')
      .then((p) => {
        if (!p) { getCurrentWindow().close().catch(() => {}); return; }
        setPayload(p);
      })
      .catch(() => getCurrentWindow().close().catch(() => {}));
  }, []);

  // 捕获完成 / 出错时关闭边框浮层
  useEffect(() => {
    const uns: Array<() => void> = [];
    listen('scroll:complete', () => getCurrentWindow().close().catch(() => {})).then((fn) => uns.push(fn));
    listen('scroll:error', () => getCurrentWindow().close().catch(() => {})).then((fn) => uns.push(fn));
    return () => { uns.forEach((fn) => fn()); };
  }, []);

  // 监听进度 → 更新右侧实时预览
  useEffect(() => {
    const uns: Array<() => void> = [];
    listen<ScrollProgress>('scroll:progress', (e) => {
      const url = e.payload.preview_data_url;
      if (url) setPreviewUrl(url);
    }).then((fn) => uns.push(fn));
    return () => { uns.forEach((fn) => fn()); };
  }, []);

  // 画选区边框 —— 所有绘制严格落在选区 [x, x+w]×[y, y+h] 之外，绝不进入被截取区域。
  // 否则 BitBlt 截帧会把遮罩/边框一起抓进去 → 长图"被框住 + 不清晰"。
  // 关键：Tauri 透明窗口是 DirectComposition 合成窗口，BitBlt 从屏幕 DC 拿到的是
  // DWM 合成后的画面，无论 SRCCOPY 还是 CAPTUREBLT 都会包含它 → 只能在绘制端避开选区。
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !payload) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    canvas.width = payload.desktop_width;
    canvas.height = payload.desktop_height;
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    const { x, y, width: w, height: h } = payload;
    const csx = canvas.clientWidth / canvas.width;
    const lw = Math.max(2, 2 / csx);
    const gap = lw; // 边框外扩量：整条边框都落在选区外

    // 主边框 —— 画在选区外侧，内边缘紧贴选区边界但不重叠
    ctx.strokeStyle = '#4A90D9';
    ctx.lineWidth = lw;
    ctx.strokeRect(x - gap, y - gap, w + gap * 2, h + gap * 2);

    // 四角加强角标 —— 向外伸出（原实现向内伸，会伸进截图区域）
    const L = 22 / csx;
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = Math.max(3, 3 / csx);
    const corners = [
      [x, y, -1, -1],
      [x + w, y, 1, -1],
      [x, y + h, -1, 1],
      [x + w, y + h, 1, 1],
    ] as Array<[number, number, number, number]>;
    for (const [cx, cy, dx, dy] of corners) {
      ctx.beginPath();
      ctx.moveTo(cx + dx * L, cy);
      ctx.lineTo(cx, cy);
      ctx.lineTo(cx, cy + dy * L);
      ctx.stroke();
    }

    // 选区标签 —— 始终画在选区外侧；上方放不下时改到下方
    const label = '📜 滚动截图区域';
    ctx.font = `${Math.max(12, 12 / csx)}px system-ui, sans-serif`;
    const tw = ctx.measureText(label).width;
    const pad = 6 / csx;
    const lx = x;
    const above = y - (24 / csx) - 2;
    const labelY = above < 0 ? y + h + 2 : above;
    ctx.fillStyle = 'rgba(74, 144, 217, 0.9)';
    ctx.fillRect(lx, labelY, tw + pad * 2, 20 / csx);
    ctx.fillStyle = '#fff';
    ctx.textBaseline = 'middle';
    ctx.fillText(label, lx + pad, labelY + (10 / csx));
  }, [payload]);

  // 右侧实时预览图（独立 img 元素，避免每次重绘 canvas）
  const previewStyle = usePreviewStyle(payload, previewUrl);

  return (
    <div style={{ width: '100vw', height: '100vh', position: 'relative', background: 'transparent' }}>
      <canvas
        ref={canvasRef}
        style={{ width: '100vw', height: '100vh', display: 'block', background: 'transparent' }}
      />
      {previewUrl && payload && (
        <div style={previewStyle}>
          <div style={{
            position: 'absolute', top: -22, left: 0, right: 0, textAlign: 'center',
            fontSize: 11, color: '#fff', background: 'rgba(74,144,217,0.9)',
            borderRadius: '4px 4px 0 0', padding: '2px 0', fontWeight: 600,
            fontFamily: 'system-ui, sans-serif', letterSpacing: 0.5,
          }}>
            实时预览
          </div>
          <img
            ref={previewRef}
            src={previewUrl}
            alt=""
            style={{
              width: '100%', height: '100%', objectFit: 'contain',
              background: 'rgba(10, 12, 30, 0.55)',
              border: '1px solid rgba(74,144,217,0.6)',
              borderRadius: 4,
            }}
          />
        </div>
      )}
    </div>
  );
}

/** 计算预览图位置：选区右侧，超出屏幕则放左侧 */
function usePreviewStyle(payload: ScrollFramePayload | null, previewUrl: string | null): React.CSSProperties {
  if (!payload || !previewUrl) return { display: 'none' };
  const { x, y, width: w, height: h, desktop_width: dw } = payload;
  const pw = 200;
  const gap = 12;
  let px = x + w + gap;
  if (px + pw > dw) px = x - pw - gap; // 右侧放不下 → 放左侧
  const py = y;
  const ph = Math.min(h, 400);
  return {
    position: 'absolute',
    left: px,
    top: py,
    width: pw,
    height: ph,
    pointerEvents: 'none',
  };
}
