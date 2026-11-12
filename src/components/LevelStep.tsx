import { useEffect, useRef, useState } from 'react';
import { KIND_LAYER, VISUAL_KINDS, type Asset, type Level, type Project } from '../types';
import { uid } from '../lib/id';

const SCALE = 2;

function cloneLevel(level: Level, w: number, h: number): Level {
  return {
    ...level,
    w,
    h,
    cells: level.cells.filter((cell) => cell.x < w && cell.y < h),
    player: { x: Math.min(level.player.x, w - 1), y: Math.min(level.player.y, h - 1) },
  };
}

export function LevelStep({
  project,
  setProject,
  onPlay,
}: {
  project: Project;
  setProject: (project: Project) => void;
  onPlay: (id: string) => void;
}) {
  const [levelId, setLevelId] = useState(project.levels[0]?.id ?? '');
  const [brush, setBrush] = useState(project.assets.find((asset) => asset.kind === 'terrain')?.id ?? '');
  const [tool, setTool] = useState<'paint' | 'erase' | 'pick'>('paint');
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const images = useRef(new Map<string, HTMLImageElement>());
  const undo = useRef<string[]>([]);
  const stroking = useRef(false);
  const projectRef = useRef(project);
  projectRef.current = project;
  const level = project.levels.find((item) => item.id === levelId) ?? project.levels[0];

  useEffect(() => {
    let cancel = false;
    for (const asset of project.assets) {
      if (!asset.src || images.current.has(asset.id)) continue;
      const image = new Image();
      image.onload = () => {
        images.current.set(asset.id, image);
        if (!cancel) draw();
      };
      image.src = asset.src;
    }
    return () => {
      cancel = true;
    };
    // draw is recreated each render; the effect only needs assets.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project.assets]);

  useEffect(() => {
    draw();
  });

  function draw() {
    const canvas = canvasRef.current;
    const current = projectRef.current.levels.find((item) => item.id === (level?.id ?? '')) ?? level;
    if (!canvas || !current) return;
    canvas.width = current.w * 16 * SCALE;
    canvas.height = current.h * 16 * SCALE;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.imageSmoothingEnabled = false;
    ctx.fillStyle = current.sky;
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    const order = ['back', 'main', 'front'] as const;
    for (const layer of order) {
      for (const cell of current.cells) {
        if (cell.layer !== layer) continue;
        const image = images.current.get(cell.assetId);
        const x = cell.x * 16 * SCALE;
        const y = cell.y * 16 * SCALE;
        if (image) ctx.drawImage(image, x, y, 16 * SCALE, 16 * SCALE);
        else {
          ctx.fillStyle = '#6a4632';
          ctx.fillRect(x, y, 16 * SCALE, 16 * SCALE);
        }
      }
    }
    const px = current.player.x * 16 * SCALE;
    const py = current.player.y * 16 * SCALE;
    const hero = images.current.get(projectRef.current.playerId || 'pip');
    if (hero) ctx.drawImage(hero, px, py, 16 * SCALE, 16 * SCALE);
    ctx.strokeStyle = '#e25b2a';
    ctx.strokeRect(px + 1, py + 1, 16 * SCALE - 2, 16 * SCALE - 2);
    ctx.strokeStyle = 'rgba(239,230,214,0.15)';
    for (let x = 0; x <= current.w; x += 1) {
      ctx.beginPath();
      ctx.moveTo(x * 16 * SCALE, 0);
      ctx.lineTo(x * 16 * SCALE, canvas.height);
      ctx.stroke();
    }
    for (let y = 0; y <= current.h; y += 1) {
      ctx.beginPath();
      ctx.moveTo(0, y * 16 * SCALE);
      ctx.lineTo(canvas.width, y * 16 * SCALE);
      ctx.stroke();
    }
  }

  function update(recipe: (level: Level) => Level) {
    if (!level) return;
    setProject({
      ...projectRef.current,
      levels: projectRef.current.levels.map((item) => (item.id === level.id ? recipe(item) : item)),
    });
  }

  function cellAt(event: React.PointerEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    if (!canvas || !level) return null;
    const rect = canvas.getBoundingClientRect();
    const x = Math.floor(((event.clientX - rect.left) * canvas.width) / rect.width / (16 * SCALE));
    const y = Math.floor(((event.clientY - rect.top) * canvas.height) / rect.height / (16 * SCALE));
    if (x < 0 || y < 0 || x >= level.w || y >= level.h) return null;
    return { x, y };
  }

  function apply(event: React.PointerEvent<HTMLCanvasElement>) {
    const pos = cellAt(event);
    if (!pos || !level) return;
    const erase = tool === 'erase' || event.button === 2;
    if (erase) {
      update((item) => ({ ...item, cells: item.cells.filter((cell) => !(cell.x === pos.x && cell.y === pos.y)) }));
      return;
    }
    if (tool === 'pick') {
      const found = [...level.cells].reverse().find((cell) => cell.x === pos.x && cell.y === pos.y);
      if (found) {
        setBrush(found.assetId);
        setTool('paint');
      }
      return;
    }
    const asset = projectRef.current.assets.find((item) => item.id === brush);
    if (!asset) return;
    if (asset.kind === 'player') {
      update((item) => ({ ...item, player: pos }));
      return;
    }
    const layer = KIND_LAYER[asset.kind];
    update((item) => ({
      ...item,
      cells: [...item.cells.filter((cell) => !(cell.x === pos.x && cell.y === pos.y && cell.layer === layer)), { x: pos.x, y: pos.y, assetId: asset.id, layer }],
    }));
  }

  function pushUndo() {
    if (!level) return;
    undo.current = [...undo.current.slice(-39), JSON.stringify(level)];
  }

  const parts = project.assets.filter((asset) => VISUAL_KINDS.includes(asset.kind));
  const music = project.assets.filter((asset) => asset.kind === 'music');

  return (
    <section className="step">
      <div className="layout">
        <aside className="panel">
          <h2>Courses</h2>
          {project.levels.map((item) => (
            <button key={item.id} type="button" className={item.id === level?.id ? 'btn primary' : 'btn'} onClick={() => setLevelId(item.id)}>
              {item.name}
            </button>
          ))}
          <button
            type="button"
            className="btn"
            onClick={() => {
              const id = uid('level');
              const terrain = project.assets.find((asset) => asset.solid);
              const cells = terrain
                ? Array.from({ length: 20 }, (_, x) => ({ x, y: 11, assetId: terrain.id, layer: 'main' as const }))
                : [];
              const created: Level = { id, name: 'New course', w: 20, h: 14, sky: '#8fb8c9', musicId: music[0]?.id, player: { x: 2, y: 10 }, cells };
              setProject({ ...project, levels: [...project.levels, created] });
              setLevelId(id);
            }}
          >
            Add course
          </button>
          {level && project.levels.length > 1 ? (
            <button
              type="button"
              className="btn ghost"
              onClick={() => {
                const next = project.levels.filter((item) => item.id !== level.id);
                setProject({ ...project, levels: next });
                setLevelId(next[0].id);
              }}
            >
              Delete course
            </button>
          ) : null}
        </aside>
        <div>
          {level ? (
            <>
              <div className="row">
                <label className="field">
                  Name
                  <input value={level.name} onChange={(event) => update((item) => ({ ...item, name: event.target.value }))} />
                </label>
                <label className="field">
                  Width
                  <input type="number" min={16} max={128} value={level.w} onChange={(event) => update((item) => cloneLevel(item, Number(event.target.value) || item.w, item.h))} />
                </label>
                <label className="field">
                  Height
                  <input type="number" min={12} max={40} value={level.h} onChange={(event) => update((item) => cloneLevel(item, item.w, Number(event.target.value) || item.h))} />
                </label>
                <label className="field">
                  Sky
                  <input type="color" value={level.sky} onChange={(event) => update((item) => ({ ...item, sky: event.target.value }))} />
                </label>
                <label className="field">
                  Music
                  <select value={level.musicId || ''} onChange={(event) => update((item) => ({ ...item, musicId: event.target.value || undefined }))}>
                    <option value="">None</option>
                    {music.map((asset) => (
                      <option key={asset.id} value={asset.id}>{asset.name}</option>
                    ))}
                  </select>
                </label>
                <button type="button" className={tool === 'paint' ? 'btn primary' : 'btn'} onClick={() => setTool('paint')}>Paint</button>
                <button type="button" className={tool === 'erase' ? 'btn primary' : 'btn'} onClick={() => setTool('erase')}>Erase</button>
                <button type="button" className={tool === 'pick' ? 'btn primary' : 'btn'} onClick={() => setTool('pick')}>Pick</button>
                <button
                  type="button"
                  className="btn"
                  onClick={() => {
                    const raw = undo.current.pop();
                    if (!raw || !level) return;
                    const restored = JSON.parse(raw) as Level;
                    setProject({ ...project, levels: project.levels.map((item) => (item.id === level.id ? restored : item)) });
                  }}
                >
                  Undo
                </button>
                <button type="button" className="btn leaf" onClick={() => onPlay(level.id)}>Play this course</button>
              </div>
              <div className="scroller" style={{ marginTop: 10 }}>
                <canvas
                  ref={canvasRef}
                  className="level-canvas"
                  data-testid="level-canvas"
                  onContextMenu={(event) => event.preventDefault()}
                  onPointerDown={(event) => {
                    pushUndo();
                    stroking.current = true;
                    event.currentTarget.setPointerCapture(event.pointerId);
                    apply(event);
                  }}
                  onPointerMove={(event) => {
                    if (stroking.current) apply(event);
                  }}
                  onPointerUp={() => {
                    stroking.current = false;
                  }}
                />
              </div>
              <div className="tray" style={{ marginTop: 10 }}>
                {parts.map((asset: Asset) => (
                  <button key={asset.id} type="button" className={brush === asset.id ? 'part on' : 'part'} onClick={() => { setBrush(asset.id); setTool('paint'); }}>
                    {asset.src ? <img src={asset.src} alt="" /> : null}
                    <div>{asset.name}</div>
                  </button>
                ))}
              </div>
            </>
          ) : <p>No courses yet.</p>}
        </div>
      </div>
    </section>
  );
}
