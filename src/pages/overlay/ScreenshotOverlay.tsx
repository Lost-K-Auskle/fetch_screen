import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface ScreenshotOverlayProps {
  imagePath: string;
  onComplete: (regionPath: string) => void;
  onCancel: () => void;
}

interface Selection {
  x: number;
  y: number;
  width: number;
  height: number;
}

export default function ScreenshotOverlay({ imagePath, onComplete, onCancel }: ScreenshotOverlayProps) {
  const [selecting, setSelecting] = useState(false);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [startPos, setStartPos] = useState({ x: 0, y: 0 });
  const [imageLoaded, setImageLoaded] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);

  // 加载全屏截图
  useEffect(() => {
    const img = new Image();
    img.onload = () => {
      imgRef.current = img;
      setImageLoaded(true);
    };
    // 使用 Tauri 的 asset:// 协议加载
    img.src = `asset://localhost/${encodeURIComponent(imagePath)}`;
  }, [imagePath]);

  // 键盘事件
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onCancel();
      }
      if (e.key === 'Enter' && selection) {
        finalizeCapture();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selection]);

  // 鼠标事件
  const getCanvasPos = useCallback((e: React.MouseEvent): { x: number; y: number } => {
    const rect = (e.target as HTMLElement).getBoundingClientRect();
    return {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
    };
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    const pos = getCanvasPos(e);
    setSelecting(true);
    setStartPos(pos);
    setSelection(null);
  }, [getCanvasPos]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!selecting) return;
    const pos = getCanvasPos(e);
    const x = Math.min(startPos.x, pos.x);
    const y = Math.min(startPos.y, pos.y);
    const width = Math.abs(pos.x - startPos.x);
    const height = Math.abs(pos.y - startPos.y);
    setSelection({ x, y, width, height });
  }, [selecting, startPos, getCanvasPos]);

  const handleMouseUp = useCallback(() => {
    setSelecting(false);
  }, []);

  // 绘制选区
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !imgRef.current || !imageLoaded) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // 绘制截图
    canvas.width = imgRef.current.naturalWidth;
    canvas.height = imgRef.current.naturalHeight;
    ctx.drawImage(imgRef.current, 0, 0);

    // 暗化遮罩
    ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // 选区（亮化）
    if (selection && selection.width > 5 && selection.height > 5) {
      ctx.drawImage(
        imgRef.current,
        selection.x, selection.y,
        selection.width, selection.height,
        selection.x, selection.y,
        selection.width, selection.height,
      );

      // 选区边框
      ctx.strokeStyle = '#4A90D9';
      ctx.lineWidth = 2;
      ctx.strokeRect(selection.x, selection.y, selection.width, selection.height);

      // 尺寸标签
      const label = `${Math.round(selection.width)} × ${Math.round(selection.height)}`;
      ctx.fillStyle = 'rgba(0, 0, 0, 0.8)';
      const textMetrics = ctx.measureText(label);
      const labelW = textMetrics.width + 12;
      const labelH = 24;
      const labelX = selection.x + selection.width - labelW;
      const labelY = selection.y + selection.height + 4;
      ctx.fillRect(labelX, labelY, labelW, labelH);
      ctx.fillStyle = '#fff';
      ctx.font = '12px system-ui';
      ctx.textBaseline = 'middle';
      ctx.fillText(label, labelX + 6, labelY + labelH / 2);

      // 放大镜 (显示选区中心像素)
      const cx = Math.round(selection.x + selection.width / 2);
      const cy = Math.round(selection.y + selection.height / 2);
      const zoomSize = 80;
      const zoomX = Math.max(0, Math.min(canvas.width - zoomSize, cx - zoomSize / 2));
      const zoomY = Math.max(0, Math.min(canvas.height - zoomSize - 60, cy - zoomSize - 60));
      ctx.drawImage(
        imgRef.current,
        cx - 5, cy - 5, 10, 10,
        zoomX, zoomY, zoomSize, zoomSize,
      );
      ctx.strokeStyle = '#4A90D9';
      ctx.lineWidth = 1;
      ctx.strokeRect(zoomX, zoomY, zoomSize, zoomSize);
      // 十字线
      ctx.strokeStyle = '#ff0';
      ctx.beginPath();
      ctx.moveTo(zoomX + zoomSize / 2, zoomY);
      ctx.lineTo(zoomX + zoomSize / 2, zoomY + zoomSize);
      ctx.moveTo(zoomX, zoomY + zoomSize / 2);
      ctx.lineTo(zoomX + zoomSize, zoomY + zoomSize / 2);
      ctx.stroke();
    }
  }, [imageLoaded, selection]);

  const finalizeCapture = useCallback(async () => {
    if (!selection || selection.width < 5 || selection.height < 5) return;
    try {
      const path = await invoke<string>('capture_region', {
        region: {
          x: Math.round(selection.x),
          y: Math.round(selection.y),
          width: Math.round(selection.width),
          height: Math.round(selection.height),
        },
      });
      onComplete(path);
    } catch (err) {
      console.error('截图裁剪失败:', err);
    }
  }, [selection, onComplete]);

  return (
    <div
      ref={containerRef}
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        width: '100vw',
        height: '100vh',
        cursor: selecting ? 'crosshair' : 'crosshair',
        zIndex: 99999,
      }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      <canvas
        ref={canvasRef}
        style={{
          width: '100%',
          height: '100%',
          objectFit: 'contain',
        }}
      />
      {!selecting && !selection && (
        <div style={{
          position: 'fixed',
          top: 20,
          left: '50%',
          transform: 'translateX(-50%)',
          background: 'rgba(0,0,0,0.8)',
          color: '#fff',
          padding: '8px 20px',
          borderRadius: 8,
          fontSize: 14,
          zIndex: 100000,
          pointerEvents: 'none',
        }}>
          拖拽选择截取区域 · Esc 取消 · Enter 确认
        </div>
      )}
    </div>
  );
}
