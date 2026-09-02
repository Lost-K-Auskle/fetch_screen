import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { emit } from '@tauri-apps/api/event';

/**
 * 全屏选区覆盖层 (Snipaste 式)
 * - 拖拽画选区
 * - 画完后可拖动选区整体移动、拖动 8 个手柄调整大小
 * - ❌/✅ 按钮在选中框外右下角
 * - 放大镜跟随光标
 */
interface OverlayPayload {
  image_path: string;
  origin_x: number; origin_y: number;
  width: number; height: number;
  image_data_url: string;
}

interface Selection { x: number; y: number; width: number; height: number; }
interface Pos { x: number; y: number; }

type ResizeHandle = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w';
type DragMode = 'idle' | 'draw' | 'move' | 'resize';

const MIN_SIZE = 10;

function inRect(p: Pos, s: Selection): boolean {
  return p.x >= s.x && p.x <= s.x + s.width && p.y >= s.y && p.y <= s.y + s.height;
}

function applyResize(orig: Selection, handle: ResizeHandle, mx: number, my: number): Selection {
  let left = orig.x, top = orig.y;
  let right = orig.x + orig.width, bottom = orig.y + orig.height;
  if (handle.includes('w')) left = Math.min(mx, right - MIN_SIZE);
  if (handle.includes('e')) right = Math.max(mx, left + MIN_SIZE);
  if (handle.includes('n')) top = Math.min(my, bottom - MIN_SIZE);
  if (handle.includes('s')) bottom = Math.max(my, top + MIN_SIZE);
  return { x: left, y: top, width: right - left, height: bottom - top };
}

function hitHandle(p: Pos, s: Selection, sx: number, sy: number): ResizeHandle | null {
  const tolX = 8 / sx, tolY = 8 / sy;
  const { x, y, width: w, height: h } = s;
  const onL = Math.abs(p.x - x) <= tolX;
  const onR = Math.abs(p.x - (x + w)) <= tolX;
  const onT = Math.abs(p.y - y) <= tolY;
  const onB = Math.abs(p.y - (y + h)) <= tolY;
  const inX = p.x > x + tolX && p.x < x + w - tolX;
  const inY = p.y > y + tolY && p.y < y + h - tolY;
  if (onT && onL) return 'nw';
  if (onT && onR) return 'ne';
  if (onB && onL) return 'sw';
  if (onB && onR) return 'se';
  if (onT && inX) return 'n';
  if (onB && inX) return 's';
  if (onL && inY) return 'w';
  if (onR && inY) return 'e';
  return null;
}

function cursorForHandle(h: ResizeHandle): string {
  switch (h) {
    case 'nw': case 'se': return 'nwse-resize';
    case 'ne': case 'sw': return 'nesw-resize';
    case 'n': case 's': return 'ns-resize';
    case 'e': case 'w': return 'ew-resize';
  }
}

export default function FullscreenOverlay() {
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [mode, setMode] = useState<DragMode>('idle');
  const [resizeHandle, setResizeHandle] = useState<ResizeHandle | null>(null);
  const [dragStart, setDragStart] = useState<Pos>({ x: 0, y: 0 });
  const [selSnapshot, setSelSnapshot] = useState<Selection | null>(null);
  const [hoverHandle, setHoverHandle] = useState<ResizeHandle | null>(null);
  const [hoverInside, setHoverInside] = useState(false);

  const bgRef = useRef<HTMLCanvasElement>(null);
  const magRef = useRef<HTMLCanvasElement>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);
  const selectionRef = useRef<Selection | null>(null);

  // 取走待显示的截图
  useEffect(() => {
    getCurrentWindow().setFocus().catch(() => {});
    window.focus();
    invoke<OverlayPayload | null>('get_overlay_payload')
      .then((payload) => {
        if (!payload) { getCurrentWindow().close().catch(() => {}); return; }
        const image = new Image();
        image.onload = () => {
          imgRef.current = image;
          setImg(image);
          getCurrentWindow().show().catch(() => {});
          getCurrentWindow().setFocus().catch(() => {});
        };
        image.onerror = () => console.error('预览图加载失败');
        image.src = payload.image_data_url;
      })
      .catch(console.error);
  }, []);

  // 同步 selection 到 ref（供 mousemove 放大镜/hover 无闭包使用）
  useEffect(() => { selectionRef.current = selection; }, [selection]);

  // CSS -> 图像像素坐标
  const toImagePos = useCallback((clientX: number, clientY: number): Pos => {
    const c = bgRef.current;
    if (!c) return { x: 0, y: 0 };
    const r = c.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return { x: 0, y: 0 };
    return { x: (clientX - r.left) * (c.width / r.width), y: (clientY - r.top) * (c.height / r.height) };
  }, []);

  // ====== 底层 canvas：遮罩 + 选区 + 8 个调整手柄（仅 selection 变化时重绘） ======
  useEffect(() => {
    const canvas = bgRef.current;
    const image = imgRef.current;
    if (!canvas || !image) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    ctx.drawImage(image, 0, 0);

    ctx.fillStyle = 'rgba(0, 0, 0, 0.45)';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    if (selection && selection.width > MIN_SIZE && selection.height > MIN_SIZE) {
      const { x, y, width: w, height: h } = selection;
      ctx.drawImage(image, x, y, w, h, x, y, w, h);
      ctx.strokeStyle = '#4A90D9';
      ctx.lineWidth = 2;
      ctx.strokeRect(x, y, w, h);

      // 尺寸标签（左上角）
      const label = `${Math.round(w)} × ${Math.round(h)}`;
      ctx.font = '12px system-ui';
      const tw = ctx.measureText(label).width;
      ctx.fillStyle = 'rgba(0, 0, 0, 0.75)';
      ctx.fillRect(x + 4, y + 4, tw + 12, 22);
      ctx.fillStyle = '#fff';
      ctx.textBaseline = 'middle';
      ctx.fillText(label, x + 10, y + 15);

      // 8 个调整手柄（角 + 边中点），大小固定约 8 CSS px
      const csx = canvas.clientWidth / canvas.width;
      const csy = canvas.clientHeight / canvas.height;
      const hwX = 4 / csx;
      const hwY = 4 / csy;
      const corners: Array<[number, number]> = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]];
      const edges: Array<[number, number]> = [[x + w / 2, y], [x + w / 2, y + h], [x, y + h / 2], [x + w, y + h / 2]];
      ctx.fillStyle = '#ffffff';
      ctx.strokeStyle = '#4A90D9';
      ctx.lineWidth = 1 / csx;
      for (const [cx, cy] of [...corners, ...edges]) {
        ctx.fillRect(cx - hwX, cy - hwY, hwX * 2, hwY * 2);
        ctx.strokeRect(cx - hwX, cy - hwY, hwX * 2, hwY * 2);
      }
    }
  }, [img, selection]);

  // ====== 顶层 canvas：放大镜 ======
  const drawMagnifier = useCallback((clientX: number, clientY: number) => {
    const canvas = magRef.current;
    const image = imgRef.current;
    const sel = selectionRef.current;
    if (!canvas || !image || !sel || sel.width < MIN_SIZE || sel.height < MIN_SIZE) return;

    const pos = toImagePos(clientX, clientY);
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    const { x: sx, y: sy, width: sw, height: sh } = sel;
    const zs = 100;
    const srcR = 8;

    let zx = sx + sw + 12;
    let zy = sy + sh - zs;
    if (zx + zs > canvas.width) zx = sx + sw - zs - 12;
    if (zy < 0) zy = sy + sh + 12;
    if (zx < 0) zx = 4;
    if (zy + zs > canvas.height) zy = canvas.height - zs - 4;

    const inSel = pos.x >= sx && pos.x <= sx + sw && pos.y >= sy && pos.y <= sy + sh;
    const mx = inSel ? pos.x : sx + sw;
    const my = inSel ? pos.y : sy;

    const srcX = Math.max(srcR, Math.min(canvas.width - srcR, mx)) - srcR;
    const srcY = Math.max(srcR, Math.min(canvas.height - srcR, my)) - srcR;

    ctx.drawImage(image, srcX, srcY, srcR * 2, srcR * 2, zx, zy, zs, zs);
    ctx.strokeStyle = '#4A90D9';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(zx, zy, zs, zs);
    ctx.strokeStyle = 'rgba(255,255,100,0.7)';
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    ctx.moveTo(zx + zs / 2, zy + 2);
    ctx.lineTo(zx + zs / 2, zy + zs - 2);
    ctx.moveTo(zx + 2, zy + zs / 2);
    ctx.lineTo(zx + zs - 2, zy + zs / 2);
    ctx.stroke();
  }, [toImagePos]);

  // 全局 mousemove：画放大镜 + 空闲态 hover 光标
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      drawMagnifier(e.clientX, e.clientY);
      const el = imgRef.current;
      const sel = selectionRef.current;
      if (mode === 'idle' && el && sel && sel.width > MIN_SIZE && sel.height > MIN_SIZE) {
        const pos = toImagePos(e.clientX, e.clientY);
        const sx = window.innerWidth / el.naturalWidth;
        const sy = window.innerHeight / el.naturalHeight;
        setHoverHandle(hitHandle(pos, sel, sx, sy));
        setHoverInside(inRect(pos, sel));
      } else {
        setHoverHandle(null);
        setHoverInside(false);
      }
    };
    window.addEventListener('mousemove', handler, { passive: true });
    return () => window.removeEventListener('mousemove', handler);
  }, [mode, toImagePos, drawMagnifier]);

  // ====== 鼠标拖拽：画 / 移动 / 缩放 ======
  const onMouseDown = (e: React.MouseEvent) => {
    if (!img) return;
    const pos = toImagePos(e.clientX, e.clientY);
    setDragStart(pos);
    if (selection && selection.width > MIN_SIZE && selection.height > MIN_SIZE) {
      const sx = window.innerWidth / img.naturalWidth;
      const sy = window.innerHeight / img.naturalHeight;
      const h = hitHandle(pos, selection, sx, sy);
      if (h) {
        setMode('resize');
        setResizeHandle(h);
        setSelSnapshot({ ...selection });
        return;
      }
      if (inRect(pos, selection)) {
        setMode('move');
        setSelSnapshot({ ...selection });
        return;
      }
    }
    // 新选区
    setSelection(null);
    setSelSnapshot(null);
    setMode('draw');
  };

  const onMouseMove = (e: React.MouseEvent) => {
    const pos = toImagePos(e.clientX, e.clientY);
    if (mode === 'draw') {
      const x = Math.min(dragStart.x, pos.x);
      const y = Math.min(dragStart.y, pos.y);
      const w = Math.abs(pos.x - dragStart.x);
      const h = Math.abs(pos.y - dragStart.y);
      setSelection({ x, y, width: w, height: h });
    } else if (mode === 'move' && selSnapshot) {
      const dx = pos.x - dragStart.x;
      const dy = pos.y - dragStart.y;
      setSelection({ x: selSnapshot.x + dx, y: selSnapshot.y + dy, width: selSnapshot.width, height: selSnapshot.height });
    } else if (mode === 'resize' && selSnapshot && resizeHandle) {
      setSelection(applyResize(selSnapshot, resizeHandle, pos.x, pos.y));
    }
  };

  const onMouseUp = () => {
    if (mode !== 'idle') setMode('idle');
    setResizeHandle(null);
  };

  // 确认 / 取消
  const finish = useCallback(async (r: Selection) => {
    try {
      const path = await invoke<string>('capture_region', {
        region: { x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height) },
      });
      await emit('region:complete', {
        path, x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height),
      });
    } catch (err) {
      console.error('裁剪失败:', err);
      await emit('region:cancelled', {});
    }
    try { await getCurrentWindow().close(); } catch { /* ignore */ }
  }, []);

  const cancel = useCallback(async () => {
    try { await emit('region:cancelled', {}); } catch { /* ignore */ }
    try { await getCurrentWindow().close(); } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.key === 'Escape') cancel();
      if (e.key === 'Enter' && selection && selection.width > MIN_SIZE && selection.height > MIN_SIZE) finish(selection);
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, [selection, finish, cancel]);

  // CSS 坐标换算比例
  const scaleX = img ? window.innerWidth / img.naturalWidth : 1;
  const scaleY = img ? window.innerHeight / img.naturalHeight : 1;

  // ❌/✅ 按钮位置：选中框外右下角（下方、贴右边缘），屏幕底部放不下则翻到上方
  const barW = 104, barH = 44;
  let btnLeft = 0, btnTop = 0;
  if (selection) {
    const right = (selection.x + selection.width) * scaleX;
    const bottom = (selection.y + selection.height) * scaleY;
    btnLeft = right - barW;
    btnTop = bottom + 8;
    if (btnTop + barH > window.innerHeight) btnTop = selection.y * scaleY - barH - 8;
    if (btnLeft < 0) btnLeft = 4;
    if (btnLeft + barW > window.innerWidth) btnLeft = window.innerWidth - barW - 4;
  }

  let cursor = 'crosshair';
  if (mode === 'move') cursor = 'move';
  else if (mode === 'resize' && resizeHandle) cursor = cursorForHandle(resizeHandle);
  else if (hoverHandle) cursor = cursorForHandle(hoverHandle);
  else if (hoverInside) cursor = 'move';

  return (
    <div
      style={{ width: '100vw', height: '100vh', cursor, position: 'relative', background: 'transparent' }}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
    >
      <canvas ref={bgRef} style={{ width: '100%', height: '100%', display: 'block', position: 'absolute', inset: 0 }} />
      <canvas ref={magRef} style={{ width: '100%', height: '100%', display: 'block', position: 'absolute', inset: 0, pointerEvents: 'none' }} />

      {!img && (
        <div style={{ position: 'fixed', top: '50%', left: '50%', transform: 'translate(-50%,-50%)', color: 'rgba(255,255,255,0.3)', fontSize: 14, zIndex: 10, pointerEvents: 'none' }}>...</div>
      )}

      {/* ❌/✅ 确认按钮：选中框外右下角 */}
      {img && selection && mode === 'idle' && selection.width > MIN_SIZE && selection.height > MIN_SIZE && (
        <div
          onMouseDown={(e) => e.stopPropagation()}
          onMouseUp={(e) => e.stopPropagation()}
          style={{
            position: 'fixed', left: btnLeft, top: btnTop,
            display: 'flex', gap: 8, zIndex: 20,
            background: 'rgba(22, 24, 44, 0.95)',
            padding: 6, borderRadius: 10,
            border: '1px solid rgba(255,255,255,0.18)',
            boxShadow: '0 2px 14px rgba(0,0,0,0.5)',
          }}
        >
          <button
            onClick={cancel}
            title="取消（Esc）"
            style={{
              width: 44, height: 32, borderRadius: 7,
              border: '1px solid rgba(255,120,120,0.45)',
              background: 'rgba(255,80,80,0.2)', color: '#ffb0b0',
              fontSize: 15, cursor: 'pointer',
              display: 'flex', alignItems: 'center', justifyContent: 'center', lineHeight: 1,
            }}
          >❌</button>
          <button
            onClick={() => finish(selection)}
            title="确认（Enter）"
            style={{
              width: 44, height: 32, borderRadius: 7,
              border: '1px solid rgba(120,255,160,0.45)',
              background: 'rgba(60,200,110,0.22)', color: '#a8f0c0',
              fontSize: 15, cursor: 'pointer',
              display: 'flex', alignItems: 'center', justifyContent: 'center', lineHeight: 1,
            }}
          >✅</button>
        </div>
      )}

      {img && mode === 'idle' && !selection && (
        <div style={{ position: 'fixed', top: 20, left: '50%', transform: 'translateX(-50%)', background: 'rgba(0,0,0,0.7)', color: '#ccc', padding: '7px 18px', borderRadius: 8, fontSize: 13, zIndex: 10, pointerEvents: 'none', border: '1px solid rgba(255,255,255,0.1)' }}>
          拖拽框选区域 · 拖动选区可移动 · 拖拽边角可调整 · Enter 确认 · Esc 取消
        </div>
      )}
    </div>
  );
}
