import {
  type CSSProperties,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

import type { ArtworkPlacement } from "../artwork";

type ArtworkCanvasProps = {
  ariaLabel: string;
  className?: string;
  dataUrl: string;
  imageAlt: string;
  imageScaleMultiplier?: number;
  onImageLoad?: (width: number, height: number) => void;
  placement: ArtworkPlacement;
};

type CanvasGeometry = {
  height: number;
  width: number;
  clips: Array<{ left: number; top: number }>;
};

const emptyGeometry: CanvasGeometry = {
  height: 0,
  width: 0,
  clips: [
    { left: 0, top: 0 },
    { left: 0, top: 0 },
  ],
};

export function ArtworkCanvas({
  ariaLabel,
  className = "",
  dataUrl,
  imageAlt,
  imageScaleMultiplier = 1,
  onImageLoad,
  placement,
}: ArtworkCanvasProps) {
  const canvasRef = useRef<HTMLDivElement>(null);
  const clipRefs = useRef<Array<HTMLDivElement | null>>([]);
  const [geometry, setGeometry] = useState(emptyGeometry);

  useLayoutEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }

    const updateGeometry = () => {
      const canvasBounds = canvas.getBoundingClientRect();
      const clips = clipRefs.current.map((clip) => {
        const clipBounds = clip?.getBoundingClientRect();
        return {
          left: clipBounds ? clipBounds.left - canvasBounds.left : 0,
          top: clipBounds ? clipBounds.top - canvasBounds.top : 0,
        };
      });

      setGeometry({
        height: canvasBounds.height,
        width: canvasBounds.width,
        clips,
      });
    };

    updateGeometry();
    const observer = new ResizeObserver(updateGeometry);
    observer.observe(canvas);
    for (const clip of clipRefs.current) {
      if (clip) {
        observer.observe(clip);
      }
    }

    return () => observer.disconnect();
  }, []);

  function imageStyle(index: number) {
    const clip = geometry.clips[index] ?? { left: 0, top: 0 };
    return {
      left: geometry.width ? `${placement.x * geometry.width - clip.left}px` : "50%",
      top: geometry.height ? `${placement.y * geometry.height - clip.top}px` : "50%",
      transform: `translate(-50%, -50%) scale(${placement.scale * imageScaleMultiplier})`,
      visibility: geometry.width && geometry.height ? "visible" : "hidden",
    } satisfies CSSProperties;
  }

  return (
    <div
      aria-label={ariaLabel}
      className={`artwork-canvas ${className}`.trim()}
      ref={canvasRef}
    >
      {["sidebar", "workspace"].map((surface, index) => (
        <div
          className={`artwork-canvas-clip artwork-canvas-clip-${surface}`}
          key={surface}
          ref={(element) => {
            clipRefs.current[index] = element;
          }}
        >
          <img
            alt={index === 0 ? imageAlt : ""}
            aria-hidden={index === 1}
            draggable={false}
            onLoad={
              index === 0 && onImageLoad
                ? (event) =>
                    onImageLoad(
                      event.currentTarget.naturalWidth,
                      event.currentTarget.naturalHeight,
                    )
                : undefined
            }
            src={dataUrl}
            style={imageStyle(index)}
          />
        </div>
      ))}
    </div>
  );
}
