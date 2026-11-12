import { useEffect, useRef } from 'react';
import { runGame } from '../runtime/player.js';
import type { Project } from '../types';

export function PlayStep({ project, courseId, onExit }: { project: Project; courseId: string | null; onExit: () => void }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return undefined;
    return runGame(canvas, project, { courseId, onExit });
  }, [project, courseId, onExit]);
  return (
    <div className="play-stage">
      <canvas ref={ref} data-testid="play-canvas" />
    </div>
  );
}
