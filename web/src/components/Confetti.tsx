"use client";

import { useEffect, useRef } from "react";
import { PALETTE } from "@/lib/puzzle";

const PIECES = 160;
const DURATION_MS = 3500;

/**
 * A burst of confetti in the clue colours over the page, for a few seconds after mount.
 * With reduced motion the pieces stay still, scattered over the page, and just fade.
 */
export function Confetti() {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const [w, h, dpr] = [innerWidth, innerHeight, devicePixelRatio || 1];
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);
    const pieces = Array.from({ length: PIECES }, (_, i) => ({
      x: still ? Math.random() * w : w * (0.3 + Math.random() * 0.4),
      y: still ? Math.random() * h : h * 0.45,
      vx: still ? 0 : (Math.random() - 0.5) * 14,
      vy: still ? 0 : -(8 + Math.random() * 12),
      angle: Math.random() * Math.PI,
      spin: still ? 0 : (Math.random() - 0.5) * 0.3,
      size: 5 + Math.random() * 6,
      color: PALETTE[i % PALETTE.length],
    }));
    const start = performance.now();
    let last = start;
    let raf = 0;
    const frame = (t: number) => {
      // Steps are in 60 fps frames, so the fall looks the same at any refresh rate.
      const dt = Math.min((t - last) / 16.7, 3);
      last = t;
      ctx.clearRect(0, 0, w, h);
      ctx.globalAlpha = Math.min(1, (DURATION_MS - (t - start)) / 800);
      for (const p of pieces) {
        if (!still) p.vy += 0.35 * dt;
        p.vx *= 0.99;
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        p.angle += p.spin * dt;
        ctx.save();
        ctx.translate(p.x, p.y);
        ctx.rotate(p.angle);
        ctx.fillStyle = p.color;
        ctx.fillRect(-p.size / 2, -p.size / 4, p.size, p.size / 2);
        ctx.restore();
      }
      raf = t - start < DURATION_MS ? requestAnimationFrame(frame) : 0;
      if (!raf) ctx.clearRect(0, 0, w, h);
    };
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  }, []);

  return (
    <canvas
      ref={ref}
      aria-hidden
      className="pointer-events-none fixed inset-0 z-50 h-full w-full"
    />
  );
}
